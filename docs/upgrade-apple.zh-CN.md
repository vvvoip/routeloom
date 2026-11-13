# 升级已有的 Apple 个人签名安装

[English](upgrade-apple.md)

本指南将能正常运行的 0.3 工作副本升级到 0.4 应用源码，同时保留本地 Xcode 签名设置。Runner 和 PacketTunnel 必须继续使用原来的 Apple 团队和 Bundle ID。不要卸载应用、删除已保存配置、钥匙串条目或加密 DNS 映射缓存。

1. 在 Mac 和 iPhone 上断开 RouteLoom；如果保护功能使 VPN 保持阻断，先选择**恢复直连互联网**。退出应用，停止 Xcode 的运行会话。
2. 在 Mac 终端执行下面整段命令。若目录不同，请修改第一行。命令会备份已跟踪文件的本地修改，只更新共享源码和构建文件；保留 `ios` 和 `macos` 目录中的项目、权限及签名设置。Git 分支/标签仍是原值，但应用版本将变为 **0.4.0+4**。

```sh
cd "$HOME/Code/routeloom-apple"
bash <<'ROUTELOOM_UPDATE'
set -euo pipefail
git fetch origin tag v0.4.0-alpha.1
mkdir -p .local
backup=".local/before-0.4-$(date +%Y%m%d-%H%M%S).patch"
git diff --binary HEAD > "$backup"
git restore --source=v0.4.0-alpha.1 --worktree -- \
  Cargo.toml Cargo.lock rust-toolchain.toml crates \
  apps/client/apple apps/client/lib apps/client/assets apps/client/test \
  apps/client/pubspec.yaml apps/client/pubspec.lock \
  scripts/build-native.sh scripts/collect-licenses.mjs scripts/bundle-client-licenses.mjs
source "$HOME/.cargo/env"
bash scripts/build-native.sh macos
bash scripts/build-native.sh ios
node scripts/collect-licenses.mjs .local/dependency-licenses
node scripts/bundle-client-licenses.mjs
cd apps/client
flutter pub get
flutter gen-l10n
flutter build macos --release --config-only
flutter build ios --release --config-only --no-codesign
grep '^version:' pubspec.yaml
open macos/Runner.xcworkspace
open ios/Runner.xcworkspace
ROUTELOOM_UPDATE
```

不要跳过 Rust 编译：新的 Apple 流量面板需要新增原生函数。如果某一步报错，停止并提供错误内容，不要粘贴私有配置。

3. 在 **macOS** 工作区检查 **PacketTunnel → Signing & Capabilities → App Sandbox**，同时勾选 **Incoming Connections (Server)** 和 **Outgoing Connections (Client)**。必须在 PacketTunnel 扩展上启用，仅在 Runner 上启用不够。保留原来的团队和 Bundle ID。在 **Product → Scheme → Edit Scheme → Run → Info** 中选择 **Release**，取消 **Debug executable**，运行目标选 **My Mac**，按 **⌘R**。
4. 在 **iOS** 工作区选择已连接且解锁的 iPhone。Runner 和 PacketTunnel 保留原团队和 Bundle ID，使用同样的 Release 运行设置，按 **⌘R** 覆盖安装，不要先删除原应用。

Mac 测试成功后，如果平时启动的是 `/Applications` 中的副本，先退出它，在 Finder 中用新生成的 `apps/client/build/macos/Build/Products/Release/routeloom.app` 替换旧副本，再从“应用程序”启动，避免继续打开旧版本。

## 应用分流策略并验证

在每台设备上分别操作：

1. 断开后进入**设置**，选择原有配置，点击**中国网站和局域网直连**。如果有自定义规则，确认替换。服务器凭据不变，并自动选择“自动”模式。
2. 连接，中间标签现在应为**流量**。访问 `www.baidu.com` 和 `www.google.com`，在流量面板搜索域名：百度应显示**直连**，Google 应显示**代理**，同时可查看匹配的规则。页面引用的 CDN 请求可能使用各自不同的路由。
3. 点击**测试连接**：此预设下 `api.ipify.org` 应通过代理，并显示服务器出口 IP。这只验证一次 HTTPS 请求，并非全应用检测。
4. 如需比较模式：断开、选择**代理**、连接并发起新请求，捕获的连接应走代理；再测试**直连**，捕获的连接和 DNS 使用本地网络，出口 IP 可能不同，部分网站可能无法访问。最后恢复**自动**模式。

旧浏览器连接可能需要新建标签页或重启浏览器才产生新记录。面板只显示捕获的 TCP/UDP；系统更具体的路由可能绕过捕获。预设是精选域名列表，不是完整地域数据库；应用自带加密 DNS 或直接使用 IP 时按 IP/默认规则处理。自动模式仍通过服务器解析 DNS。完整行为见[客户端设置](client-features.zh-CN.md)。
