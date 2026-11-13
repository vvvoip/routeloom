# Platform matrix / 平台能力表

All five platforms have capture implementations. This is a developer milestone, not a signed, device-validated release. 所有平台均已加入捕获代码；仍需签名、真机测试和生产打包。

| Platform | Implemented capture | Current validation | Remaining release gates |
|---|---|---|---|
| Windows | Wintun + Rust TLS helper | Compilation and Rust tests | Real adapter/routes/DNS, installed privileged service, signing |
| macOS | PacketTunnelProvider + native TLS | Unsigned Xcode build | Apple signing, VPN entitlement, real-device lifecycle, distribution |
| Linux | TUN + Rust TLS helper | Isolated IPv4/IPv6 TUN integration | Resolver variants, service privilege boundary, real-network testing |
| Android | VpnService + native TLS | Debug APK build | Physical devices, permission/revocation, background/network changes |
| iOS/iPadOS | PacketTunnelProvider + native TLS | Unsigned Xcode build | Apple signing, physical device, TestFlight/distribution |

Check the latest CI run for build outcomes; this table describes the checks, not a permanent promise that every commit passes. The shared English/Chinese UI imports profiles and reports native status. [Setup and verification](vpn-development.md) · [中文说明](vpn-development.zh-CN.md).

Domain rules and PAC are not applied to the VPN packet path yet. iOS per-app rules require managed devices. Some applications, containers, VMs and other active VPNs require special treatment. Bundle identifiers are provisional until signing setup. Android requires API 29+; Linux DNS integration currently requires systemd-resolved.

References: [Apple deployment](https://developer.apple.com/documentation/technotes/tn3134-network-extension-provider-deployment), [Android VPN](https://developer.android.com/develop/connectivity/vpn).
