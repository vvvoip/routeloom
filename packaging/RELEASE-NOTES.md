RouteLoom 0.4.0 alpha 1 brings the traffic dashboard, Auto/Proxy/Direct modes and exit-IP testing to macOS and iOS. All five clients include English and Simplified Chinese controls and a **China and LAN direct** routing preset.

- Inspect destination domains/IPs, actual direct/proxy/blocked routes, matching rules, connection states and transferred bytes. Search/filter, pause the display and clear completed rows. The latest 256 connection records stay in RAM; no payloads or credentials are recorded.
- Auto applies your rules. Proxy and Direct override every rule, including block rules. Direct also resolves DNS over the underlying network and works without the proxy server. Disconnect before changing modes.
- Test connection uses api.ipify.org through the active engine and displays its actual route and observed exit IP. This is a single-request test, not a complete leak audit.
- In Settings, disconnect and apply **China and LAN direct** to the selected profile. Local networks, .cn and listed mainland services go direct; other destinations use the server. Credentials and other settings are preserved. Custom rules require confirmation before replacement. This is a curated preset, not a complete GeoIP database. Auto DNS still resolves through the server.
- Includes the macOS extension's incoming-network entitlement fix and credential-safe Apple startup diagnostics. Existing profiles and the deployed RL/2 server remain compatible. Apple monitoring has bounded messages and timeouts so a restarted extension does not stall polling.
- Existing Apple installations must rebuild the native engine and app using the same team and bundle identifiers. See [Apple upgrade instructions](https://github.com/vvvoip/routeloom/blob/main/docs/upgrade-apple.md).

| Platform | Download | Requirements |
| --- | --- | --- |
| Windows | routeloom-windows-x64.zip | Windows 10/11 x64; extract all files and run Start-RouteLoom.cmd |
| Linux | routeloom-linux-amd64.deb | Ubuntu 24.04+ amd64 desktop, systemd-resolved, polkit and a system keyring |
| Android | routeloom-android.apk | Android 10+; same permanent signing key as 0.2 |
| macOS / iOS | routeloom-apple-signing-kit.zip | Source signing kit; Xcode, Flutter 3.50.0, Rust 1.100.0, Node.js, Apple Developer team; **not an installable DMG/IPA** |

Read [installation](https://github.com/vvvoip/routeloom/blob/main/docs/install.md) and [modes, traffic, rules and recovery](https://github.com/vvvoip/routeloom/blob/main/docs/client-features.md). Import a private profile; no server credentials are included.

Protection is opt-in. Desktop protection survives disconnect; use **Restore direct internet** before uninstalling. Android requires system Always-on VPN and Block connections without VPN. Explicit Direct mode/rules are intentional exceptions while connected. Existing more-specific routes can bypass capture without protection. Domains require RouteLoom DNS or its local proxy; app-managed encrypted DNS/literal IPs may show only IPs. The encrypted DNS mapping cache persists separately from RAM-only connection history.

Updates are publisher-signed and hash-verified. Windows binaries do not have a publisher code-signing certificate. Android 0.2 upgrades retain the signing key; 0.1 requires saving the profile and uninstalling the old preview once. Personally signed Apple apps still need Xcode for each update. Physical Android/Apple testing and complete sleep/network-transition testing remain pending. This remains an experimental alpha.

简体中文：0.4 版本为 macOS 和 iOS 加入流量面板、自动/代理/直连模式及出口 IP 测试。五个平台都提供“中国网站和局域网直连”预设；在设置中断开后应用，保留服务器凭据和其他设置。该预设并非完整地理位置数据库，自动模式 DNS 仍通过服务器解析。Apple 用户请使用相同团队和 Bundle ID 重新编译原生引擎及应用，参阅[升级指南](https://github.com/vvvoip/routeloom/blob/main/docs/upgrade-apple.zh-CN.md)。面板仅显示捕获的流量，真机与网络切换验证仍需继续。
