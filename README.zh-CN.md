# RouteLoom

[English](README.md) · [简体中文](README.zh-CN.md) · [官方网站](https://vvvoip.github.io/routeloom/zh-CN/)

RouteLoom 是一个正在开发的开源、自托管代理项目：共享路由核心、五大平台客户端，以及由你管理的服务端。

**当前处于基础开发阶段（pre-alpha），尚未实现 VPN 隧道，也没有可供生产使用的安装包。** 首个公开可用版本将同时面向 Windows、macOS、Linux、Android 和 iOS/iPadOS，并提供英语与简体中文。

## 项目组成

- Rust 工作区：确定性规则引擎、命令行工具，以及仅监听回环地址的服务端健康检查原型。
- Flutter 客户端：双语界面与平台能力说明；原生适配和传输尚未完成，因此连接功能保持不可用。
- 静态双语官网：使用 Node.js 构建，通过 GitHub Actions 发布到 GitHub Pages。

阅读[开发指南](docs/development.zh-CN.md)、[开发路线](docs/roadmap.md)和[平台能力表](docs/platforms.md)。构建成功不代表 VPN 功能已经完成。

## 原则

规则和 DNS 策略明确可见，使用成熟加密方案，每台设备独立凭据，默认不记录浏览历史，准确说明平台限制。计划中的自托管产品不要求注册中央账户。

## 许可证

原创代码采用 Apache-2.0；第三方组件保留各自许可证。安全问题请参阅 [SECURITY.md](SECURITY.md)。
