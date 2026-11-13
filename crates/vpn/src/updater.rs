//! Windows portable-package updater. Runs from a copied helper after verification.
use routeloom_tunnel::{invalid, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    archive: PathBuf,
    sha256: String,
    target: PathBuf,
    pid: u32,
}
fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.components().all(|p| matches!(p, Component::Normal(_)))
        && !path.to_string_lossy().contains(':')
        && !path
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with("routeloom-state")
        && !path.to_string_lossy().contains(".routeloom-update")
}
fn no_links(path: &Path, root: &Path) -> Result<()> {
    use std::os::windows::fs::MetadataExt;
    let mut cursor = path;
    loop {
        if let Ok(meta) = fs::symlink_metadata(cursor) {
            if meta.file_attributes() & 0x400 != 0 {
                return Err(invalid("update path contains a reparse point"));
            }
        }
        if cursor == root {
            break;
        }
        cursor = cursor
            .parent()
            .ok_or_else(|| invalid("update path escapes application"))?;
    }
    Ok(())
}
pub fn run() -> Result<()> {
    let mut raw = String::new();
    std::io::stdin().take(65537).read_to_string(&mut raw)?;
    if raw.len() > 65536 {
        return Err(invalid("update request too large"));
    }
    let request: Request =
        serde_json::from_str(&raw).map_err(|_| invalid("invalid update request"))?;
    let target = fs::canonicalize(&request.target)?;
    if target.parent().is_none()
        || !target.join("routeloom.exe").is_file()
        || !target.join("routeloom-vpn.exe").is_file()
    {
        return Err(invalid("invalid application directory"));
    }
    no_links(&target, &target)?;
    if request.sha256.len() != 64 || !request.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid("invalid update digest"));
    }
    let source = fs::File::open(&request.archive)?;
    if source.metadata()?.len() > 262144000 {
        return Err(invalid("update too large"));
    }
    let mut bytes = Vec::new();
    source.take(262144001).read_to_end(&mut bytes)?;
    if format!("{:x}", Sha256::digest(&bytes)) != request.sha256 {
        return Err(invalid("update hash mismatch"));
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| invalid("invalid update archive"))?;
    if archive.len() > 5000 {
        return Err(invalid("too many update entries"));
    }
    let stage = target.join(format!(".routeloom-update-{}", std::process::id()));
    fs::create_dir(&stage)?;
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let mut total = 0u64;
    let extracted = (|| -> Result<()> {
        for i in 0..archive.len() {
            let entry = archive
                .by_index(i)
                .map_err(|_| invalid("invalid zip entry"))?;
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| invalid("unsafe zip path"))?;
            if !safe_relative(&relative)
                || entry
                    .unix_mode()
                    .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(invalid("unsafe update entry"));
            }
            if entry.is_dir() {
                continue;
            }
            if !seen.insert(relative.to_string_lossy().to_ascii_lowercase()) {
                return Err(invalid("duplicate update entry"));
            }
            total = total
                .checked_add(entry.size())
                .ok_or_else(|| invalid("update size overflow"))?;
            if total > 1_073_741_824 {
                return Err(invalid("expanded update too large"));
            }
            let out = stage.join("new").join(&relative);
            fs::create_dir_all(out.parent().unwrap())?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&out)?;
            let expected = entry.size();
            let written = std::io::copy(&mut entry.take(expected + 1), &mut file)?;
            if written != expected {
                return Err(invalid("expanded entry size mismatch"));
            }
            file.sync_all()?;
            no_links(&target.join(&relative), &target)?;
            entries.push(relative);
        }
        for required in [
            "routeloom.exe",
            "routeloom-vpn.exe",
            "flutter_windows.dll",
            "wintun.dll",
        ] {
            if !seen.contains(required) {
                return Err(invalid("incomplete application package"));
            }
        }
        Ok(())
    })();
    if let Err(error) = extracted {
        cleanup(&stage, &target)?;
        return Err(error);
    }
    println!("{{\"state\":\"update-ready\"}}");
    std::io::stdout().flush()?;
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_OBJECT_0},
            System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
        };
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, request.pid);
        if !process.is_null() {
            let result = WaitForSingleObject(process, 30000);
            CloseHandle(process);
            if result != WAIT_OBJECT_0 {
                cleanup(&stage, &target)?;
                return Err(invalid("application did not exit"));
            }
        }
    }
    let mut changed: Vec<(PathBuf, bool)> = Vec::new();
    let installed = (|| -> Result<()> {
        for relative in &entries {
            let destination = target.join(relative);
            no_links(&destination, &target)?;
            fs::create_dir_all(destination.parent().unwrap())?;
            let backup = stage.join("old").join(relative);
            let existed = destination.exists();
            if existed {
                fs::create_dir_all(backup.parent().unwrap())?;
                fs::rename(&destination, &backup)?;
            }
            changed.push((relative.clone(), existed));
            fs::rename(stage.join("new").join(relative), destination)?;
        }
        Ok(())
    })();
    if installed.is_err() {
        for (relative, existed) in changed.into_iter().rev() {
            let destination = target.join(&relative);
            if destination.is_file() {
                fs::remove_file(&destination)?;
            }
            if existed {
                fs::rename(stage.join("old").join(relative), destination)?;
            }
        }
    }
    cleanup(&stage, &target)?;
    let _ = std::process::Command::new(target.join("routeloom.exe"))
        .current_dir(&target)
        .spawn();
    installed
}
fn cleanup(stage: &Path, target: &Path) -> Result<()> {
    let resolved = fs::canonicalize(stage)?;
    let root = fs::canonicalize(target)?;
    if resolved.parent() != Some(root.as_path())
        || !resolved
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(".routeloom-update-")
    {
        return Err(invalid("unsafe update cleanup"));
    }
    fs::remove_dir_all(resolved)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_paths_are_confined() {
        for p in [
            "../routeloom.exe",
            "C:\\Windows\\x",
            "routeloom.exe:stream",
            "routeloom-state/cache",
            ".routeloom-update-1/file",
        ] {
            assert!(!safe_relative(Path::new(p)));
        }
        assert!(safe_relative(Path::new(
            "data/flutter_assets/AssetManifest.bin"
        )));
    }
}
