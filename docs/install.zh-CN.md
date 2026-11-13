# 安装 RouteLoom alpha
[English](install.md) · 简体中文

从 [GitHub Releases](https://github.com/vvvoip/routeloom/releases) 的 **Assets** 下载对应文件。需要你自己的服务器设备配置；安装包不包含共享账号或令牌。

## Windows 10/11 x64
1. 下载 routeloom-windows-x64.zip，**完整解压**到一个新文件夹。
2. 双击 Start-RouteLoom.cmd，接受管理员权限提示。当前 alpha 的界面和辅助程序都以管理员身份运行；取消提示不会启动 VPN。
3. 私下打开 windows.profile.json，复制完整 JSON，在客户端选择“从剪贴板导入配置”。
4. 点击“连接”，等待“已连接”，然后测试浏览器和终端。关闭前先“断开连接”。

ZIP 已包含辅助程序、已签名的 Wintun DLL 和微软 C++ 运行库。不要直接在压缩包中运行，也不要将 EXE 与数据/DLL 分开。RouteLoom 自身尚未代码签名，可能触发 SmartScreen；请校验下载哈希，不要关闭系统安全防护。正式版的独立服务提权和签名安装程序尚待完成。

## Ubuntu 24.04+ 桌面版 amd64
下载 routeloom-linux-amd64.deb，在该文件所在目录执行：

```sh
sudo apt install ./routeloom-linux-amd64.deb
```

从应用菜单启动 RouteLoom，导入 linux.profile.json，点击连接，在桌面授权窗口中认证。只有网络辅助程序以 root 运行，界面保持普通用户权限。需要桌面的 polkit 认证代理和运行中的 systemd-resolved；不会配置免密 sudo。

该包要求 glibc 2.39，不能在 Debian 12 桌面上运行；其他发行版及架构尚未提供安装包。独立服务器仍支持 Debian 12。

卸载前先断开，然后运行 `sudo apt remove routeloom`。没有桌面认证代理时，可用终端运行 `sudo /usr/lib/routeloom/routeloom-vpn connect /完整路径/linux.profile.json`，Ctrl-C 断开。

## Android 10+
下载并打开 routeloom-android-preview.apk，根据系统提示允许下载应用安装 APK。打开 RouteLoom，复制并导入 android.profile.json，点击连接并接受 VPN 系统权限。常驻通知中可断开连接。

这是使用预览密钥签名的 release 构建，不是 Play 商店正式版。后续 alpha 可能更换签名，系统拒绝覆盖安装时需卸载旧预览版。单独保存私有配置，以便重新导入。

## macOS 和 iOS/iPadOS
Apple 下载项是**源码签名包，不是可直接安装的 DMG/IPA**。VPN 扩展需要你的 Apple 开发者签名和描述文件。不要通过关闭 SIP/Gatekeeper 代替正确签名。

在 Mac 上安装 Xcode 和 Flutter 3.50.0，登录 Apple 开发者账户，解压签名包后运行：

```sh
cd routeloom
bash scripts/prepare-apple.sh YOURTEAMID com.yourname.routeloom
```

将 YOURTEAMID 替换为 10 位 Team ID，并填写唯一的反向域名包名。脚本会设置两个应用及扩展的身份，准备依赖并打开工作区。在 Xcode 选择 Mac 或连接的 iPhone/iPad，检查 **Runner 和 PacketTunnel** 的 Signing & Capabilities（包括 Network Extensions），然后 Run。导入对应设备配置并授权 VPN。

此流程用于个人开发测试；TestFlight/App Store 和 macOS 签名分发尚未发布。详见 [Apple 开发说明](vpn-development.zh-CN.md)。

## 使用与限制
每台设备使用独立配置。不要公开令牌、配置或截图。配置只保留在当前应用会话中，重开后需重新导入。界面支持英语和简体中文。

连接后检查公网出口是否为服务器 IP，测试浏览器和终端，再断开并确认网络恢复。服务器没有 IPv6 出口时，IPv6 会被捕获但不能访问互联网。

alpha 尚未实现持久断线保护、自动重连、保存配置、自动更新，以及 VPN 中的域名/PAC 分流。断开或故障会释放捕获；已有更具体的系统路由仍可能直连。它还不是完成的隐私保护产品。

使用 Windows 的 Get-FileHash、Linux 的 sha256sum 或 macOS 的 shasum -a 256，将下载文件与同一发布版本的 SHA256SUMS.txt 对比。
