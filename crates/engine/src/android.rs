use jni::{
    objects::{JByteArray, JClass, JObject, JString, JValue},
    sys::{jbyteArray, jint, jlong},
    JNIEnv,
};
use std::sync::Arc;
#[no_mangle]
pub extern "system" fn Java_org_routeloom_routeloom_NativeEngine_start(
    mut env: JNIEnv,
    _: JClass,
    profile: JString,
    service: JObject,
) -> jlong {
    let result = (|| {
        let text: String = env.get_string(&profile).ok()?.into();
        let vm = env.get_java_vm().ok()?;
        let service = env.new_global_ref(service).ok()?;
        let protect = Arc::new(move |socket: &socket2::Socket| {
            use std::os::fd::AsRawFd;
            let mut env = vm
                .attach_current_thread()
                .map_err(|_| routeloom_tunnel::invalid("JNI attach failed"))?;
            let ok = env
                .call_method(
                    service.as_obj(),
                    "protect",
                    "(I)Z",
                    &[JValue::Int(socket.as_raw_fd())],
                )
                .and_then(|v| v.z())
                .unwrap_or(false);
            if ok {
                Ok(())
            } else {
                Err(routeloom_tunnel::invalid("VPN socket protection failed"))
            }
        });
        Some(crate::ffi::start_json(&text, protect) as jlong)
    })();
    result.unwrap_or(0)
}
#[no_mangle]
pub extern "system" fn Java_org_routeloom_routeloom_NativeEngine_write(
    env: JNIEnv,
    _: JClass,
    id: jlong,
    data: JByteArray,
) {
    if let Ok(data) = env.convert_byte_array(data) {
        crate::ffi::write(id as u64, data);
    }
}
#[no_mangle]
pub extern "system" fn Java_org_routeloom_routeloom_NativeEngine_read(
    env: JNIEnv,
    _: JClass,
    id: jlong,
) -> jbyteArray {
    let data = crate::ffi::read(id as u64).unwrap_or_default();
    env.byte_array_from_slice(&data)
        .map(|v| v.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_org_routeloom_routeloom_NativeEngine_status(
    _: JNIEnv,
    _: JClass,
    id: jlong,
) -> jint {
    crate::ffi::routeloom_status(id as u64) as jint
}
#[no_mangle]
pub extern "system" fn Java_org_routeloom_routeloom_NativeEngine_stop(
    _: JNIEnv,
    _: JClass,
    id: jlong,
) {
    crate::ffi::routeloom_stop(id as u64);
}
