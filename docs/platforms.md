# Platform matrix / 平台能力表

Version 0.2 shares a Rust TCP/UDP policy engine and English/Chinese Flutter settings across all five platforms. This is an experimental alpha, not a complete device-validated release.

| Platform | Capture and protection | Distribution | Validation still needed |
|---|---|---|---|
| Windows | Wintun, protected sockets, persistent WFP rules | x64 ZIP; administrator GUI/helper | Sleep/wake, other VPNs, reboot scenarios; publisher code signing and service separation |
| Linux | TUN, marked sockets, owned nftables table and boot unit | Ubuntu 24.04+ amd64 DEB | Resolver variants, distro support, reboot and network transitions |
| Android | VpnService/JNI; system always-on and lockdown | Permanently signed Android 10+ APK | Physical devices, OS revocation, battery and background transitions |
| macOS | PacketTunnel + Rust static library; system protection | Source signing kit; user signs in Xcode | Signed device runtime, entitlement/distribution checks |
| iOS/iPadOS | PacketTunnel + Rust static library; system protection | Source signing kit; user signs in Xcode | Physical device, sleep/network transitions, signed distribution |

Build and integration results are recorded in GitHub Actions and [validation notes](milestone-2.md). Shared features include domain/IP/port/protocol routing, loopback PAC/HTTP CONNECT, reconnect, saved profiles and signed update checks. Apple personal builds require Xcode for installation and updates.

Domain classification requires RouteLoom DNS or the local proxy. Opaque encrypted DNS/literal IP traffic uses IP/default rules. Geographic datasets and per-process routing are not implemented. Some containers/VMs, managed per-app VPNs and competing tunnels need platform-specific work. Linux currently requires systemd-resolved and a system Secret Service keyring.

中文：五个平台共用分流、重连、配置和更新检查功能；Windows/Linux 使用系统防火墙实现持久保护，Android 需手动启用系统始终 VPN/阻断，Apple 使用系统 VPN 保护且仍需自行签名。编译通过不代表真机、重启和全部网络切换场景已经验证。详见[中文功能与限制](client-features.zh-CN.md)。
