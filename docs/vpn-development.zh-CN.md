# VPN 开发里程碑

[English](vpn-development.md) · 简体中文

当前已加入 TLS 1.3 数据包服务器、Windows/Linux TUN 辅助程序、Android VpnService，以及 macOS/iOS PacketTunnelProvider。仍为开发版本；未签名编译成功不代表真机 VPN 验证通过。

## 服务器

使用 Rust 1.100 执行 `cargo build --release -p routeloom-vpn --locked`。Linux 需要 OpenSSL 3、iproute2 和 /dev/net/tun。

```sh
node scripts/create-dev-profile.mjs .local/demo 服务器IP:9443 tunnel.example
sudo target/release/routeloom-vpn serve .local/demo/server.json
```

生成器创建有效期 30 天的开发证书、私钥、设备配置及仅存储令牌哈希的服务器配置。请私密传输配置，不要提交到 Git。设备 IPv4 地址范围为 10.77.0.2–254，IPv6 为 fd77::2–ffff；每台设备使用独立地址和随机令牌。

服务器创建 rl-server 接口。访问互联网还需要配置转发和防火墙/NAT，见[部署说明](../deploy/vpn.md)。CI 使用隔离网络命名空间测试，无需云服务器。

## 客户端

- Windows：需要管理员权限及辅助程序旁的签名 Wintun DLL。在管理员终端运行 `routeloom-vpn connect device.profile.json`。
- Linux：需要 root、iproute2、systemd-resolved/resolvectl。运行 `sudo routeloom-vpn connect device.profile.json`。其他 DNS 管理器尚未接入。
- Windows/Linux Flutter 界面：将辅助程序放在应用旁，或通过 ROUTELOOM_VPN_BINARY 指定绝对路径。它继承当前权限，不会自动提权。建议先用命令行验证；生产版仍需最小权限后台服务和安装器。
- Android：最低 Android 10/API 29。安装调试 APK，复制设备配置，选择“从剪贴板导入配置”，点击“连接”并授予 VPN 权限。前台通知支持断开；撤销权限会关闭连接。暂不支持开机自动连接。
- macOS/iOS：在 Mac 上打开 Runner Xcode workspace，为 Runner 和 PacketTunnel 两个 target 设置开发团队、唯一的应用标识和 Network Extensions 权限。扩展标识必须是应用标识加 `.PacketTunnel`。使用 Mac 和实体 iPhone/iPad 验证。CI 不包含签名和真机运行验证；分发方式必须符合 Apple 要求，Developer ID system extension 方案尚未实现。

配置只在本次会话中使用，未写入 VPN 偏好设置。TLS 校验证书和服务器名称后才发送令牌、安装路由。断开时回滚拥有的路由和 DNS；**连接失败后释放捕获，不提供持久断网保护**。进程被强制结束后应检查残留网络设置。

## 验证和范围

`cargo test --workspace --locked` 检查 TLS、认证、数据包、租约和路由排除。`sudo bash scripts/test-vpn-linux.sh` 使用真实 TUN 接口验证 IPv4/IPv6 往返及清理；测试中的 DNS 命令为桩实现，不能证明 DNS 防泄漏。Flutter 测试覆盖配置校验及连接界面。

发布前必须完成五个平台的权限、TCP/UDP/DNS、IPv6、错误证书/令牌、服务器断连、睡眠唤醒、网络切换及 VPN 共存测试。0.2 已加入共享 RL/2 域名/IP 分流、PAC、自动重连、持久保护控制、安全配置和签名更新检查，详见[功能与恢复指南](client-features.zh-CN.md)。地理规则、自动注册、完整真机验证及正式安装器仍待完成。协议详见 [RL/2](rl2.md)，服务端保留 RL/1 兼容。
