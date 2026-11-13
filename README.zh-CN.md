# RouteLoom

[English](README.md) · [简体中文](README.zh-CN.md) · [官方网站](https://vvvoip.github.io/routeloom/zh-CN/)

RouteLoom 是一个正在开发的开源、自托管代理项目：共享路由核心、五大平台客户端，以及由你管理的服务端。

**当前处于实验性 VPN alpha。** [下载客户端预览版](https://github.com/vvvoip/routeloom/releases)，并阅读[安装说明](docs/install.zh-CN.md)。提供 Windows、Ubuntu 和 Android 安装包；Apple 平台目前需要源码签名包及你的开发者账户。界面包含英语和简体中文。完整签名的五平台正式版本仍是目标。

## 项目组成

- Rust 工作区：规则/PAC 命令行、经过认证的 IPv4/IPv6 传输、Linux 服务端及 Windows/Linux 捕获辅助程序。
- Flutter 客户端：双语配置导入和连接控制、Android VpnService，以及 macOS/iOS Network Extension targets。
- 静态双语官网：使用 Node.js 构建，通过 GitHub Actions 发布到 GitHub Pages。

先阅读 [VPN 开发指南](docs/vpn-development.zh-CN.md)。域名规则尚未接入 VPN 数据路径。

阅读[开发指南](docs/development.zh-CN.md)、[开发路线](docs/roadmap.md)和[平台能力表](docs/platforms.md)。构建成功不代表 VPN 功能已经完成。

## 原则

规则和 DNS 策略明确可见，使用成熟加密方案，每台设备独立凭据，默认不记录浏览历史，准确说明平台限制。计划中的自托管产品不要求注册中央账户。

## 许可证

原创代码采用 Apache-2.0；第三方组件保留各自许可证。安全问题请参阅 [SECURITY.md](SECURITY.md)。
