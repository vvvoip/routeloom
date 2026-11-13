Download the asset for your platform and follow [installation / 安装说明](https://github.com/vvvoip/routeloom/blob/main/docs/install.md).

| Platform | Download | Requirements |
| --- | --- | --- |
| Windows | routeloom-windows-x64.zip | Windows 10/11 x64; extract all files and run Start-RouteLoom.cmd; accept UAC |
| Linux | routeloom-linux-amd64.deb | Ubuntu 24.04+ amd64 desktop, systemd-resolved, a polkit authentication agent |
| Android | routeloom-android-preview.apk | Android 10+; allow installation from your download app |
| macOS / iOS | routeloom-apple-signing-kit.zip | Source signing kit; Xcode, Flutter 3.50.0, Apple Developer team; **not an installable app** |

Import your private device profile, connect and grant the operating system permission. No server credentials are included in downloads. Windows bundles the VPN helper, signed Wintun runtime and Microsoft C++ runtime. Linux elevates only the helper through polkit. English and Simplified Chinese are included.

This is an **experimental alpha**, not the completed five-platform public release. Apple signed distribution and physical-device validation remain pending. Domain routing/PAC integration in the VPN, persistent kill switch, automatic reconnect, protected profile persistence and automatic updates are still pending. Disconnect/failure releases capture; more-specific OS routes may remain direct.

Windows binaries do not have a RouteLoom code-signing certificate; its alpha GUI runs elevated. Android uses an ephemeral preview signing key: future builds may require uninstalling this preview. Do not use this signing identity for a store release. Checksums are in SHA256SUMS.txt. Server certificate renewal remains manual.

简体中文：下载 Windows ZIP、Linux DEB 或 Android APK 后，按[中文安装说明](https://github.com/vvvoip/routeloom/blob/main/docs/install.zh-CN.md)导入私有设备配置并连接。macOS/iOS 提供的是源码签名包，不是可直接安装的应用，仍需在自己的 Mac 上使用 Xcode 和开发者账户签名。本版本是实验性 alpha；域名规则、断线保护、自动重连、配置持久化和自动升级尚未完成。
