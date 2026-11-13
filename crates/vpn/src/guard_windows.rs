use routeloom_tunnel::Result;
use std::{
    os::windows::ffi::OsStrExt,
    ptr::{null, null_mut},
};
use windows_sys::{
    core::GUID,
    Win32::{
        Foundation::HANDLE,
        NetworkManagement::{
            IpHelper::ConvertInterfaceAliasToLuid, Ndis::NET_LUID_LH, WindowsFilteringPlatform::*,
        },
    },
};
const PROVIDER: GUID = GUID::from_u128(0x14a43a18_3b20_4b51_b147_b7a13c756100);
const SUBLAYER: GUID = GUID::from_u128(0x14a43a18_3b20_4b51_b147_b7a13c756101);
fn key(i: u128) -> GUID {
    GUID::from_u128(0x14a43a18_3b20_4b51_b147_b7a13c757000 + i)
}
fn check(code: u32) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(code as i32))
    }
}
struct Engine(HANDLE);
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            FwpmTransactionAbort0(self.0);
            FwpmEngineClose0(self.0);
        }
    }
}
fn open() -> Result<Engine> {
    unsafe {
        let mut h = null_mut();
        check(FwpmEngineOpen0(null(), 10, null(), null(), &mut h))?;
        check(FwpmTransactionBegin0(h, 0))?;
        Ok(Engine(h))
    }
}
unsafe fn clear(h: HANDLE) -> Result<()> {
    for i in 0..32 {
        let code = FwpmFilterDeleteByKey0(h, &key(i));
        if code != 0 && code != 0x80320003 {
            check(code)?;
        }
    }
    Ok(())
}
fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
pub fn disable() -> Result<()> {
    unsafe {
        let engine = open()?;
        clear(engine.0)?;
        for code in [
            FwpmSubLayerDeleteByKey0(engine.0, &SUBLAYER),
            FwpmProviderDeleteByKey0(engine.0, &PROVIDER),
        ] {
            if code != 0 && code != 0x80320007 && code != 0x80320005 {
                check(code)?;
            }
        }
        check(FwpmTransactionCommit0(engine.0))
    }
}
pub fn enable(name: &str) -> Result<()> {
    unsafe {
        let engine = open()?;
        let h = engine.0;
        clear(h)?;
        let mut label = wide(std::ffi::OsStr::new("RouteLoom persistent traffic guard"));
        let display = FWPM_DISPLAY_DATA0 {
            name: label.as_mut_ptr(),
            description: null_mut(),
        };
        let mut provider_key = PROVIDER;
        let provider = FWPM_PROVIDER0 {
            providerKey: PROVIDER,
            displayData: display,
            flags: FWPM_PROVIDER_FLAG_PERSISTENT,
            ..Default::default()
        };
        let code = FwpmProviderAdd0(h, &provider, null_mut());
        if code != 0 && code != 0x80320009 {
            check(code)?;
        }
        let sub = FWPM_SUBLAYER0 {
            subLayerKey: SUBLAYER,
            displayData: display,
            flags: FWPM_SUBLAYER_FLAG_PERSISTENT,
            providerKey: &mut provider_key,
            weight: 0xfffe,
            ..Default::default()
        };
        let code = FwpmSubLayerAdd0(h, &sub, null_mut());
        if code != 0 && code != 0x80320009 {
            check(code)?;
        }
        let mut luid: NET_LUID_LH = std::mem::zeroed();
        check(ConvertInterfaceAliasToLuid(
            wide(std::ffi::OsStr::new(name)).as_ptr(),
            &mut luid,
        ))?;
        let mut interface = luid.Value;
        let exe = wide(std::env::current_exe()?.as_os_str());
        let mut app = null_mut();
        check(FwpmGetAppIdFromFileName0(exe.as_ptr(), &mut app))?;
        let result = (|| -> Result<()> {
            for (family, layer) in [
                FWPM_LAYER_ALE_AUTH_CONNECT_V4,
                FWPM_LAYER_ALE_AUTH_CONNECT_V6,
            ]
            .into_iter()
            .enumerate()
            {
                let mut index = (family * 16) as u128;
                let mut add =
                    |mut conditions: Vec<FWPM_FILTER_CONDITION0>, allow: bool| -> Result<()> {
                        let mut filter = FWPM_FILTER0 {
                            filterKey: key(index),
                            displayData: display,
                            flags: FWPM_FILTER_FLAG_PERSISTENT,
                            providerKey: &mut provider_key,
                            layerKey: layer,
                            subLayerKey: SUBLAYER,
                            ..Default::default()
                        };
                        index += 1;
                        filter.weight = FWP_VALUE0 {
                            r#type: FWP_UINT8,
                            Anonymous: FWP_VALUE0_0 {
                                uint8: if allow { 15 } else { 0 },
                            },
                        };
                        filter.action.r#type = if allow {
                            FWP_ACTION_PERMIT
                        } else {
                            FWP_ACTION_BLOCK
                        };
                        filter.numFilterConditions = conditions.len() as u32;
                        filter.filterCondition = conditions.as_mut_ptr();
                        check(FwpmFilterAdd0(h, &filter, null_mut(), null_mut()))
                    };
                add(
                    vec![condition(
                        FWPM_CONDITION_ALE_APP_ID,
                        FWP_BYTE_BLOB_TYPE,
                        FWP_CONDITION_VALUE0_0 { byteBlob: app },
                    )],
                    true,
                )?;
                add(
                    vec![condition(
                        FWPM_CONDITION_IP_LOCAL_INTERFACE,
                        FWP_UINT64,
                        FWP_CONDITION_VALUE0_0 {
                            uint64: &mut interface,
                        },
                    )],
                    true,
                )?;
                let mut loopback = condition(
                    FWPM_CONDITION_FLAGS,
                    FWP_UINT32,
                    FWP_CONDITION_VALUE0_0 {
                        uint32: FWP_CONDITION_FLAG_IS_LOOPBACK,
                    },
                );
                loopback.matchType = FWP_MATCH_FLAGS_ALL_SET;
                add(vec![loopback], true)?;
                add(
                    vec![
                        condition(
                            FWPM_CONDITION_IP_PROTOCOL,
                            FWP_UINT8,
                            FWP_CONDITION_VALUE0_0 { uint8: 17 },
                        ),
                        condition(
                            FWPM_CONDITION_IP_LOCAL_PORT,
                            FWP_UINT16,
                            FWP_CONDITION_VALUE0_0 {
                                uint16: if family == 0 { 68 } else { 546 },
                            },
                        ),
                        condition(
                            FWPM_CONDITION_IP_REMOTE_PORT,
                            FWP_UINT16,
                            FWP_CONDITION_VALUE0_0 {
                                uint16: if family == 0 { 67 } else { 547 },
                            },
                        ),
                    ],
                    true,
                )?;
                if family == 1 {
                    for icmp in 133..=136 {
                        add(
                            vec![
                                condition(
                                    FWPM_CONDITION_IP_PROTOCOL,
                                    FWP_UINT8,
                                    FWP_CONDITION_VALUE0_0 { uint8: 58 },
                                ),
                                condition(
                                    FWPM_CONDITION_IP_LOCAL_PORT,
                                    FWP_UINT16,
                                    FWP_CONDITION_VALUE0_0 { uint16: icmp },
                                ),
                            ],
                            true,
                        )?;
                    }
                }
                add(vec![], false)?;
            }
            check(FwpmTransactionCommit0(h))
        })();
        FwpmFreeMemory0((&mut app as *mut *mut FWP_BYTE_BLOB).cast());
        result
    }
}
fn condition(
    field_key: GUID,
    kind: FWP_DATA_TYPE,
    value: FWP_CONDITION_VALUE0_0,
) -> FWPM_FILTER_CONDITION0 {
    FWPM_FILTER_CONDITION0 {
        fieldKey: field_key,
        matchType: FWP_MATCH_EQUAL,
        conditionValue: FWP_CONDITION_VALUE0 {
            r#type: kind,
            Anonymous: value,
        },
    }
}

pub fn self_test() -> Result<()> {
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        return Err(std::io::Error::other(
            "Guard test is restricted to an ephemeral GitHub runner",
        ));
    }
    let executable = std::env::current_exe()?;
    let mut config = tun::Configuration::default();
    config
        .tun_name("RouteLoom-GuardTest")
        .address((10, 77, 0, 253))
        .netmask((255, 255, 255, 0))
        .up();
    config.platform_config(|p| {
        p.wintun_file(executable.with_file_name("wintun.dll"));
    });
    let _device = tun::create(&config).map_err(std::io::Error::other)?;
    fn probe() -> Result<bool> {
        Ok(std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command",
            "$ErrorActionPreference='Stop'; $probe=[Net.Sockets.TcpClient]::new(); try { if ($probe.ConnectAsync('1.1.1.1',443).Wait(4000)) { exit 0 } else { exit 1 } } catch { exit 1 } finally { $probe.Dispose() }"
        ]).output()?.status.success())
    }
    if !probe()? {
        return Err(std::io::Error::other("Guard test baseline egress failed"));
    }
    use std::os::windows::process::CommandExt;
    let mut watchdog = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 25; & $env:ROUTELOOM_GUARD_HELPER unlock",
        ])
        .env("ROUTELOOM_GUARD_HELPER", &executable)
        .creation_flags(0x08000000)
        .spawn()?;
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = disable();
        }
    }
    let restore = Restore;
    let applied = std::process::Command::new(&executable)
        .args(["guard-enable", "RouteLoom-GuardTest"])
        .status()?
        .success();
    if !applied {
        return Err(std::io::Error::other("Persistent filter install failed"));
    }
    let blocked = !probe()?;
    disable()?;
    drop(restore);
    let _ = watchdog.kill();
    let _ = watchdog.wait();
    if !blocked || !probe()? {
        return Err(std::io::Error::other(
            "Guard block/recovery verification failed",
        ));
    }
    println!("Persistent WFP filters survived the installing process and blocked external egress; unlock restored it.");
    Ok(())
}
