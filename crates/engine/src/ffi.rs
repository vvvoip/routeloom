//! Bounded C ABI. The caller owns all buffers; no Rust allocation crosses the ABI.
use crate::{socket::Protector, Engine};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
struct Entry {
    engine: Mutex<Engine>,
    input: tokio::sync::mpsc::Sender<Vec<u8>>,
    state: Arc<std::sync::atomic::AtomicU8>,
}
static ENGINES: OnceLock<Mutex<HashMap<u64, Arc<Entry>>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);
fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime")
    })
}
pub fn start_json(json: &str, protect: Protector) -> u64 {
    let Ok(directory) = crate::cache::default_directory() else {
        return 0;
    };
    start_json_in(json, protect, directory)
}
pub fn start_json_in(json: &str, protect: Protector, directory: std::path::PathBuf) -> u64 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if json.len() > 65536 {
            return 0;
        }
        let Ok(profile) = serde_json::from_str(json) else {
            return 0;
        };
        let _guard = runtime().enter();
        let Ok(engine) = crate::start_in(profile, protect, directory) else {
            return 0;
        };
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut all = ENGINES.get_or_init(Default::default).lock().unwrap();
        if !all.is_empty() {
            return 0;
        } // One device VPN engine per process.
        let entry = Entry {
            input: engine.input.clone(),
            state: engine.state.clone(),
            engine: Mutex::new(engine),
        };
        all.insert(id, Arc::new(entry));
        id
    }))
    .unwrap_or(0)
}
fn engine(id: u64) -> Option<Arc<Entry>> {
    ENGINES
        .get_or_init(Default::default)
        .lock()
        .ok()?
        .get(&id)
        .cloned()
}
pub fn write(id: u64, data: Vec<u8>) -> bool {
    if data.len() > 1280 {
        return false;
    }
    engine(id)
        .map(|e| e.input.try_send(data).is_ok())
        .unwrap_or(false)
}
pub fn read(id: u64) -> Option<Vec<u8>> {
    let engine = engine(id)?;
    let mut engine = engine.engine.lock().ok()?;
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_millis(100), engine.output.recv())
            .await
            .ok()
            .flatten()
    })
}
#[no_mangle]
pub extern "C" fn routeloom_status(id: u64) -> u8 {
    engine(id)
        .map(|e| e.state.load(Ordering::Acquire))
        .unwrap_or(2)
}
#[no_mangle]
pub extern "C" fn routeloom_stop(id: u64) {
    if let Ok(mut all) = ENGINES.get_or_init(Default::default).lock() {
        if let Some(engine) = all.remove(&id) {
            drop(all);
            if let Ok(mut engine) = engine.engine.lock() {
                if let Some(stop) = engine.stop.take() {
                    let _ = stop.send(());
                }
                runtime().block_on(async {
                    let _ = (&mut engine.task).await;
                });
            }
        }
    }
}
/// # Safety
/// JSON must address len readable bytes. protect must remain callable until stop returns.
#[no_mangle]
pub unsafe extern "C" fn routeloom_start(
    json: *const u8,
    len: usize,
    protect: Option<extern "C" fn(i32, usize) -> bool>,
    context: usize,
) -> u64 {
    if json.is_null() || len > 65536 {
        return 0;
    }
    let Ok(text) = std::str::from_utf8(std::slice::from_raw_parts(json, len)) else {
        return 0;
    };
    let protector: Protector = Arc::new(move |socket, _destination| {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            if let Some(callback) = protect {
                if !callback(socket.as_raw_fd(), context) {
                    return Err(routeloom_tunnel::invalid("socket protection failed"));
                }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (socket, protect, context);
        }
        Ok(())
    });
    start_json(text, protector)
}
/// # Safety
/// data must address len readable bytes.
#[no_mangle]
pub unsafe extern "C" fn routeloom_write(id: u64, data: *const u8, len: usize) -> bool {
    if data.is_null() || len > 1280 {
        return false;
    }
    write(id, std::slice::from_raw_parts(data, len).to_vec())
}
/// # Safety
/// output must address capacity writable bytes.
#[no_mangle]
pub unsafe extern "C" fn routeloom_read(id: u64, output: *mut u8, capacity: usize) -> usize {
    if output.is_null() || capacity < 1280 {
        return 0;
    }
    let Some(data) = read(id) else { return 0 };
    if data.len() > capacity {
        return 0;
    }
    std::ptr::copy_nonoverlapping(data.as_ptr(), output, data.len());
    data.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_is_independent_of_output_and_restart_releases_cache() {
        let directory =
            std::env::temp_dir().join(format!("routeloom-ffi-{}", rand::random::<u64>()));
        let profile = serde_json::json!({
            "endpoint":"127.0.0.1:9", "server_name":"localhost", "ca_pem":"test",
            "device":"ffi-test", "token":"a".repeat(64), "ipv4":"10.77.0.2","ipv6":"fd77::2",
            "routes":["0.0.0.0/0"],"dns":["1.1.1.1"],
            "options":{"policy":"final proxy:default","pac_port":0}
        })
        .to_string();
        let protect: Protector = Arc::new(|_, _| Ok(()));
        let first = start_json_in(&profile, protect.clone(), directory.clone());
        assert_ne!(first, 0);
        let entry = engine(first).unwrap();
        let output_lock = entry.engine.lock().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            let result = write(first, vec![0; 20]);
            let status = routeloom_status(first);
            tx.send((result, status)).unwrap();
        });
        let result = rx.recv_timeout(Duration::from_secs(1));
        drop(output_lock);
        writer.join().unwrap();
        assert!(result.expect("input/status must not wait on output").0);
        routeloom_stop(first);
        drop(entry);
        let second = start_json_in(&profile, protect, directory.clone());
        assert_ne!(
            second, 0,
            "stop must release the encrypted journal before returning"
        );
        routeloom_stop(second);
        assert!(std::fs::canonicalize(&directory)
            .unwrap()
            .starts_with(std::fs::canonicalize(std::env::temp_dir()).unwrap()));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
