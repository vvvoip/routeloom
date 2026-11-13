# 客户端设置
[English](client-features.md) · [安装说明](install.zh-CN.md)

0.2 版本在 Windows、macOS、Linux、Android 和 iOS/iPadOS 提供相同的英语/简体中文设置。Apple 客户端仍需使用你的 Xcode 开发者签名。这是实验性 alpha，编译通过不等于真机验证完成。

## 保存配置与分流

从剪贴板导入私有配置后，应用会将其保存到系统保护的凭据存储。在“已保存配置”中选择、重命名、编辑或删除配置。安全存储不可用时会明确提示，并允许仅在本次会话使用。Linux 需要解锁支持 Secret Service 的密钥环。Android 和 Apple 还会为后台 VPN 保存受保护的配置副本。

修改配置和规则前先断开。规则从上到下匹配，首条命中生效，最后必须有且仅有一条 final：

```text
domain ads.example.com block
domain-suffix example.com direct
cidr 192.168.0.0/16 direct
port 25 block
network udp proxy:default
final proxy:default
```

支持精确域名、域名后缀、IPv4/IPv6 CIDR、端口和 TCP/UDP；动作支持 direct、block、proxy:default。其他代理组、地理位置数据库和应用进程规则尚未实现。

共享引擎将捕获的 TCP/UDP 转换为连接流。RouteLoom DNS 为域名分配虚拟地址，使共用同一真实 IP 的域名仍能应用不同规则。域名通过加密服务器连接解析，包括最终直连的域名。因此，仅本地 DNS 知道的内部域名尚不支持；这类内部资源可使用 IP 规则。应用自带加密 DNS 或直接使用 IP 时，只能按 IP/默认规则匹配。域名使用 ASCII/punycode。

本地加密 DNS 映射会跨重启保留。它不是访问历史日志，但确实保存加密域名映射。每个配置最多 4,096 个域名，所有配置累计最多 65,534 次分配。未知、损坏或耗尽的映射会拒绝连接，不会把旧地址分配给另一域名。应用仍缓存虚拟 DNS 地址时，不要删除该文件。

当前仅合成 A/AAAA 记录，其他 DNS 类型返回空答案。不转发原始 ICMP 或其他非 TCP/UDP 协议。系统已有更具体的路由可能绕过捕获；开启断线保护可阻止非预期直连。

## PAC 与终端

连接期间，默认 PAC 地址为 http://127.0.0.1:12789/proxy.pac。可从应用复制到支持 PAC 的浏览器自动代理设置。同一端口支持 HTTP 和 CONNECT。终端示例：

```sh
export http_proxy=http://127.0.0.1:12789
export https_proxy=http://127.0.0.1:12789
```

PAC 将请求交给本地引擎，由它执行完整规则，不会自动回退到 DIRECT。不执行导入的任意 JavaScript PAC，也不支持 SOCKS5。不再使用时请移除浏览器/终端代理设置；断开会关闭本地监听。在配置中将 pac_port 设为 0 可禁用监听。

## 自动重连与持久保护

自动重连采用约 1 至 30 秒的指数退避与随机延迟。网络或服务器不可用时仍保留捕获，现有连接关闭，应用需要重试。引擎不会自行改为直连。

保护默认关闭。连接前启用断线保护；**普通“断开连接”不会移除持久保护**。要恢复普通网络，请使用“恢复直接上网”。

- Windows 使用独立的持久 WFP 规则，辅助程序退出后继续生效。允许 RouteLoom 辅助程序、回环、隧道接口和必要网络配置流量。保护从启用时开始，依赖系统防火墙服务。
- Linux 使用独立 nftables 表及 systemd 启动单元，不删除其他防火墙表。允许带标记的 RouteLoom 套接字、隧道、回环及必要网络配置流量。
- Android 必须在系统 VPN 设置中启用“始终开启的 VPN”和“阻止未使用 VPN 的连接”。应用内开关不能代替系统授权。恢复直连需要关闭系统阻断。
- Apple 使用 Network Extension 的 includeAllNetworks 和按需重连。启用保护后断开会暂停转发并保留捕获。系统网络例外仍适用；不承诺开机或所有状态切换期间绝无流量。个人签名版本仍需真机验证。

连接时显式 direct 规则属于有意放行。睡眠唤醒、网络切换、系统更新、其他 VPN 及容器还需要进一步测试。

无法使用界面时：

```powershell
# 管理员 PowerShell，在 Windows 解压目录中：
.\routeloom-vpn.exe unlock
```
```sh
# Linux：
sudo /usr/lib/routeloom/routeloom-vpn unlock
```

**卸载前先恢复直接上网**。只删除应用不会清除桌面持久防火墙规则。

## 更新

启用后，应用打开期间每六小时检查 GitHub。下载前验证固定的 Ed25519 发布者签名、版本序号及 SHA-256；拒绝回退版本、替换文件名或修改后的安装包。

Windows 在应用退出后替换文件，保留状态目录，失败时尝试回滚。Linux 使用系统安装器打开验证后的 DEB。Android 打开系统 APK 安装器，可能需要允许 RouteLoom 安装应用。安装仍需要你的操作或系统确认。

五个平台都会检查更新。个人签名的 macOS/iOS 需要下载新的签名包并在 Xcode 重新签名；无人值守 Apple 二进制更新需要配置正式签名分发渠道。Apple 下载项是源码，不是可直接安装的更新包。

0.1 Android 预览版使用不同签名。请先保存私有配置，卸载旧预览版，再安装 0.2 并重新导入；后续使用固定项目签名密钥。Windows 尚无发布者代码签名证书。
