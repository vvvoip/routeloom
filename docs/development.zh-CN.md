# 开发指南

工具版本：Rust 1.100.0、Flutter 3.50.0、Node.js 24。Windows 原生编译需要 Visual Studio C++ 工具；Apple 平台需要 macOS 和 Xcode；Android 需要 SDK 和 JDK。

## Rust 规则工具

```sh
cargo test --workspace
cargo run -p routeloom-cli -- --lang zh-CN check examples/default.rules
cargo run -p routeloom-cli -- --lang zh-CN explain examples/default.rules blocked.example
cargo run -p routeloom-cli -- pac examples/browser.rules 127.0.0.1:7890
```

规则按顺序匹配，必须以 final 结束。支持 domain、domain-suffix、cidr、port、network。动作是 direct、block、proxy:名称。域名使用 ASCII 或 Punycode。引擎不会自行解析 DNS。命令行开发诊断暂为英语。

PAC 导出仅支持能够保留语义的域名规则，不会忽略不支持的规则或自动附加直连后备。导出 PAC 不代表已经启动本地代理。

## 客户端

```sh
node scripts/bootstrap-client.mjs
cd apps/client
flutter pub get
flutter gen-l10n
flutter test
flutter run
```

脚本生成五大平台的官方 Flutter 工程，且不会覆盖现有原生适配文件。目前界面不会请求 VPN 权限；连接按钮保持不可用。语言切换仅在本次运行期间有效。翻译位于 lib/l10n/app_en.arb 和 app_zh.arb。

## 官网

```sh
node website/build.mjs
node --test scripts/site.test.mjs
```

产物位于 website/dist。GitHub 仓库 Settings → Pages → Source 选择 GitHub Actions，main 上的官网变更会触发发布。

当前服务端仅有回环地址健康检查原型，尚不能代理流量，也未部署云服务器。界面测试及原生构建通过，不代表 VPN 接管、商店分发或真机测试已完成。
