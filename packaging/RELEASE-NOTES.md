RouteLoom 0.2.0 alpha 1 adds domain/IP/port/protocol routing, a local HTTP/CONNECT proxy and PAC, automatic reconnect, protected saved profiles, persistent protection controls, and signed update checks. English and Simplified Chinese are available on all five client platforms.

| Platform | Download | Requirements |
| --- | --- | --- |
| Windows | routeloom-windows-x64.zip | Windows 10/11 x64; extract all files and run Start-RouteLoom.cmd |
| Linux | routeloom-linux-amd64.deb | Ubuntu 24.04+ amd64 desktop, systemd-resolved, polkit and a system keyring |
| Android | routeloom-android.apk | Android 10+; permanent project signing key |
| macOS / iOS | routeloom-apple-signing-kit.zip | Source signing kit; Xcode, Flutter 3.50.0, Rust 1.100.0, Node.js, Apple Developer team; **not an installable DMG/IPA** |

Follow [installation](https://github.com/vvvoip/routeloom/blob/main/docs/install.md) and [settings, rules and recovery](https://github.com/vvvoip/routeloom/blob/main/docs/client-features.md). Import a private profile; no server credentials are included.

Protection is opt-in. Desktop protection survives disconnect and requires **Restore direct internet** before uninstalling. Android requires system Always-on VPN and Block connections without VPN. Apple uses system VPN controls with OS exceptions. Apple signing, physical mobile-device validation and complete sleep/network-transition testing remain pending.

Updates are publisher-signed and hash-verified. Windows can replace its portable package, while Linux/Android use their OS installer. Personally signed Apple apps still need Xcode for each update. Windows binaries remain unsigned by a publisher certificate. If upgrading from the 0.1 Android preview, save your profile and uninstall that preview once because the signing key changed.

Domain routing requires RouteLoom DNS or its local proxy. App-managed encrypted DNS/literal IPs use IP/default rules. The engine handles TCP/UDP, synthesizes A/AAAA records, and has bounded persistent DNS mappings; see the settings guide for limits. The server supports new RL/2 flows and legacy RL/1 clients. Server certificate renewal remains manual.

简体中文：本版本加入域名/IP/端口/协议分流、本地 HTTP/CONNECT 代理及 PAC、自动重连、安全保存配置、持久保护控制和签名更新检查。五个平台包含英语和简体中文。请阅读[安装说明](https://github.com/vvvoip/routeloom/blob/main/docs/install.zh-CN.md)和[功能/恢复指南](https://github.com/vvvoip/routeloom/blob/main/docs/client-features.zh-CN.md)。桌面端卸载前先“恢复直接上网”；Android 需在系统中开启始终 VPN 和阻断；Apple 仍需自己在 Xcode 签名，更新也需重新签名。0.1 Android 预览版需先保存配置、卸载旧版再安装。真机验证及正式分发仍待完成。
