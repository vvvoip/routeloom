# Platform matrix / 平台能力表

All five platforms are launch targets. **No native VPN adapter is implemented yet.** Generated Flutter runners build an interface only.

| Platform | Planned capture | Initial state | Release prerequisites |
|---|---|---|---|
| Windows | Signed Wintun + background service | Contract + shared UI | Driver distribution review, installer, route/DNS rollback, signing |
| macOS | Network Extension | Contract + shared UI | macOS/Xcode, entitlements, signing, lifecycle tests |
| Linux | TUN + systemd | Contract + shared UI | Privilege boundary, distributions, DNS integration |
| Android | VpnService | Contract + shared UI | Consent, foreground lifecycle, per-app rules, physical device tests |
| iOS/iPadOS | Packet Tunnel Provider | Contract + shared UI | Mac/Xcode, entitlement feasibility, physical device, distribution |

Shared launch baseline: supported TCP/UDP tunneling, explicit routes, DNS behavior, server selection, device enrollment, English and Simplified Chinese.

Per-app rules are platform dependent. iOS per-app VPN requires managed devices. PAC is for proxy-aware applications, not arbitrary mobile traffic. Some applications, containers, VMs, and another active VPN require special treatment. Application bundles and build identifiers are provisional until signing setup.

Apple references: [Network Extension deployment](https://developer.apple.com/documentation/technotes/tn3134-network-extension-provider-deployment), [App Review VPN rules](https://developer.apple.com/app-store/review/guidelines/#vpn-apps).
