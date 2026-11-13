RouteLoom 0.3.0 alpha 1 adds a live TCP/UDP connection dashboard, Auto/Proxy/Direct routing modes, and an explicit HTTPS exit-IP test for Windows, Android and Linux. All new controls support English and Simplified Chinese.

- Inspect destination domains/IPs, actual direct/proxy/blocked routes, matching rules, connection states and transferred bytes. Search/filter, pause the display and clear completed rows. The latest 256 connection records stay in RAM; no payloads or credentials are recorded.
- Auto applies your rules. Proxy and Direct override every rule, including block rules. Direct also resolves DNS over the underlying network and works without the proxy server. Disconnect before changing modes.
- Test connection uses api.ipify.org through the active engine and displays its actual route and observed exit IP. This is a single-request test, not a complete leak audit.
- Existing profiles and the deployed RL/2 server remain compatible. The new dashboard UI is not yet available on Apple; Apple work resumes separately.

| Platform | Download | Requirements |
| --- | --- | --- |
| Windows | routeloom-windows-x64.zip | Windows 10/11 x64; extract all files and run Start-RouteLoom.cmd |
| Linux | routeloom-linux-amd64.deb | Ubuntu 24.04+ amd64 desktop, systemd-resolved, polkit and a system keyring |
| Android | routeloom-android.apk | Android 10+; same permanent signing key as 0.2 |
| macOS / iOS | routeloom-apple-signing-kit.zip | Source signing kit; Xcode, Flutter 3.50.0, Rust 1.100.0, Node.js, Apple Developer team; **not an installable DMG/IPA** |

Read [installation](https://github.com/vvvoip/routeloom/blob/main/docs/install.md) and [modes, traffic, rules and recovery](https://github.com/vvvoip/routeloom/blob/main/docs/client-features.md). Import a private profile; no server credentials are included.

Protection is opt-in. Desktop protection survives disconnect; use **Restore direct internet** before uninstalling. Android requires system Always-on VPN and Block connections without VPN. Explicit Direct mode/rules are intentional exceptions while connected. Existing more-specific routes can bypass capture without protection. Domains require RouteLoom DNS or its local proxy; app-managed encrypted DNS/literal IPs may show only IPs. The encrypted DNS mapping cache persists separately from RAM-only connection history.

Updates are publisher-signed and hash-verified. Windows binaries do not have a publisher code-signing certificate. Android 0.2 upgrades retain the signing key; 0.1 requires saving the profile and uninstalling the old preview once. Personally signed Apple apps still need Xcode for each update. Physical Android/Apple testing and complete sleep/network-transition testing remain pending. This remains an experimental alpha.

简体中文：本版本为 Windows、Android 和 Linux 增加 TCP/UDP 实时连接面板、自动/代理/直连模式及 HTTPS 出口 IP 测试，均支持中英文。面板显示实际路由、规则和流量；最多 256 条连接记录仅在内存中。自动模式按规则分流；代理/直连会覆盖全部规则（包括阻断），切换前需断开。直连 DNS 通过底层网络，无需代理服务器可用。“测试连接”访问 api.ipify.org，不是完整泄漏检测。已有配置及服务器兼容。Android 0.2 可保留数据升级；Apple 新界面及真机验证待后续完成。请阅读[安装说明](https://github.com/vvvoip/routeloom/blob/main/docs/install.zh-CN.md)和[模式、流量与恢复指南](https://github.com/vvvoip/routeloom/blob/main/docs/client-features.zh-CN.md)。
