# Apple 客户安装与发布

[English](apple-customer-distribution.md) · 简体中文

RouteLoom 面向客户的发布路径是：**先通过 TestFlight 测试，再发布到 iPhone/iPad 的 App Store 和 Mac 的 Mac App Store**。客户通过安装链接获取应用，无需 Xcode、Rust、Flutter、自己的开发者账户或“开发者模式”。由发布者为每次发布完成构建和签名。[Apple 分发方式](https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases)

## 当前状态

GitHub 上的 Apple ZIP 仍是源码签名包。此次修改没有发布公开的 RouteLoom TestFlight、App Store、Mac App Store 或已签名 DMG 下载。生成本地归档不等于提供了客户安装包；未签名的 CI 构建也不能证明分发签名及真机行为正确。

开发版已在所有者的 Mac 和 iPhone 上成功连接，但更新后的 Mac 流量面板与连接测试仍待真机确认。睡眠唤醒、切换网络、覆盖升级以及在另一位客户设备上的安装也需验证后才能公开发布。准备正式发布时，请保留现有个人测试安装。

## 发布账户与应用设置

1. 确认发布团队以**组织**身份加入 Apple Developer Program。Apple 的 VPN 审核规则要求组织账户；个人开发证书能在自己的设备运行，并不代表满足公开发布条件。如果目前是个人账户，请先向 Apple 处理组织注册问题，再承诺公开分发 VPN 应用。[审核规则 5.4](https://developer.apple.com/app-store/review/guidelines/#vpn-apps)
2. 选定发布者拥有的固定 Bundle ID，并建立匹配的 App Store Connect 应用记录和平台条目。Runner 与其 `.PacketTunnel` 扩展必须使用同一团队，并正确配置 Network Extensions 与钥匙串权限。检查**两个平台上的两个 target**的签名和描述文件。在 Xcode 分发流程中使用对应的 Apple 分发签名资产。[配置 Network Extensions](https://developer.apple.com/documentation/xcode/configuring-network-extensions/)
3. 更新现有安装时，保持原团队、Bundle ID 和钥匙串组。如果从个人团队迁移到组织团队，可能产生独立应用身份；不能假定新应用能读取旧应用的已保存配置。请保留私有配置备份，明确安排迁移。
4. 准备真实的英语和简体中文商店介绍、截图、支持信息、年龄分级及隐私信息。请发布者确认打包的 RouteLoom 图标，并准备商店要求的截图。发布者必须提供并确认自己的身份、联系方式、实际数据处理方式和隐私政策网址；仓库示例不等于已完成的隐私政策。成功生成归档不代表已经具备提交审核的全部条件。

签名凭据由发布者保管。不要将 Apple 密码、签名私钥、App Store Connect API 私钥或私有服务器配置粘贴到聊天、公开 issue 或 GitHub 仓库。本地使用 Xcode 签名，无需将这些凭据导出到 CI。

## 隐私与审核准备

应用内声明及公开隐私政策必须符合实际发布版本，包括所选服务器的作用、设备上保存的凭据、内存中显示的连接元数据、加密 DNS 映射存储，以及可选的外部诊断请求。准确说明发布者的服务器运营方式，不要声称第三方服务器也遵循相同做法。Apple 要求在使用 VPN 服务前展示数据声明，并对 VPN 数据处理作出相应承诺；发布者需根据真实业务审阅这些内容。[VPN 隐私要求](https://developer.apple.com/app-store/review/guidelines/#vpn-apps)

根据实际实现及拟发布地区回答 App Store Connect 的加密问题。RouteLoom 包含 Rust TLS/加密库和本地加密 DNS 缓存，并非仅调用操作系统 HTTPS API。未确认符合条件前，不要填入 `ITSAppUsesNonExemptEncryption = false`。是否需要文件或分类，应由发布者核实自身义务后确定。[Apple 加密出口合规说明](https://developer.apple.com/help/app-store-connect/manage-app-information/overview-of-export-compliance/)

选择发布地区，并按 Apple 对相应地区的要求提供许可信息。本指南不判断当地许可资格。通过私有审核字段向 App Review 提供独立、可用的审核配置和操作说明。使用可撤销的专用测试凭据，审核期间保持测试服务器可用；不要将凭据放入发布归档或公开文档。[审核准备](https://developer.apple.com/app-store/review/)

## 更新已有个性化签名的源码目录

如果 Mac 上的源码仍停留在旧 tag，或此前通过选择性复制文件升级，请先执行本节。**只有客户分发相关修改合并进 `main` 后，才能使用此流程。** 命令会先确认拉取的分支包含归档脚本，否则停止。全新获取当前 `main` 的目录无需执行这个更新块。

先断开 Mac 和 iPhone 上的 RouteLoom；如果保护设置阻止断开，请使用**恢复直接上网**。退出应用，停止 Xcode 运行会话。在仓库任意子目录中完整执行以下命令。它会备份已跟踪文件的改动，更新共享客户端、原生代码、构建脚本和已提交的图标，并只修改每个平台扩展 Info.plist 的两个版本字段。原有 Xcode 项目、签名设置、entitlements、钥匙串组和已保存配置会保留。

```sh
bash <<'ROUTELOOM_STORE_UPDATE'
set -euo pipefail
routeloom_root="$(git rev-parse --show-toplevel)"
cd "$routeloom_root"
test -f apps/client/pubspec.yaml
command -v node
git fetch origin main
git cat-file -e 'origin/main:scripts/archive-apple-store.sh'
mkdir -p .local
routeloom_backup="$(mktemp -d "$routeloom_root/.local/before-apple-store-$(date +%Y%m%d-%H%M%S)-XXXXXX")"
git diff --binary HEAD > "$routeloom_backup/tracked-edits.patch"
git restore --source=origin/main --worktree -- \
  Cargo.toml Cargo.lock rust-toolchain.toml crates packaging/release.json \
  apps/client/apple apps/client/lib apps/client/assets apps/client/test \
  apps/client/pubspec.yaml apps/client/pubspec.lock \
  apps/client/ios/Runner/Assets.xcassets/AppIcon.appiconset \
  apps/client/macos/Runner/Assets.xcassets/AppIcon.appiconset \
  scripts/build-native.sh scripts/collect-licenses.mjs \
  scripts/bundle-client-licenses.mjs scripts/archive-apple-store.sh \
  scripts/apple-store-export.py
routeloom_version="$(node -p "JSON.parse(require('fs').readFileSync('packaging/release.json','utf8')).version")"
routeloom_build_number="$(node -p "JSON.parse(require('fs').readFileSync('packaging/release.json','utf8')).sequence")"
for platform in macos ios; do
  /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $routeloom_version" "apps/client/$platform/PacketTunnel/Info.plist"
  /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $routeloom_build_number" "apps/client/$platform/PacketTunnel/Info.plist"
done
echo "Tracked-edit backup: $routeloom_backup/tracked-edits.patch"
grep '^version:' apps/client/pubspec.yaml
ROUTELOOM_STORE_UPDATE
```

命令执行后，终端仍可能显示原来的 tag 和有改动的工作区，这是保留签名项目的预期结果。请将备份留在本地。任一命令失败后，先解决该错误再构建。下面的归档脚本会从更新后的源码重新编译 Rust 库和应用，不要复用旧应用包。

## 在发布者的 Mac 上生成归档

Mac 目标在生成签名归档前，应先在 Xcode 的 **Runner → General → App Category** 设置应用类别（此客户端适合 Utilities／工具）。这会将 `LSApplicationCategoryType` 写入本地应用配置。未签名 CI 可以在未设置类别时完成编译，但这不代表 Mac App Store 所需元数据已经齐全。

使用更新后的源码，并安装 Xcode、Flutter、Rust/rustup 和 Node.js。先在 Xcode 登录发布团队，为 Runner 与 PacketTunnel 配置签名。将发布者确认的隐私政策发布到真实、公开可访问的 HTTPS 网址，并将下面占位符替换为该网址；私有草稿或示例政策不能用于发布。构建命令保留已有项目身份。在仓库任意子目录中执行：

```sh
cd "$(git rev-parse --show-toplevel)"
export ROUTELOOM_PRIVACY_URL='<published HTTPS privacy-policy URL>'
bash scripts/archive-apple-store.sh ios
bash scripts/archive-apple-store.sh macos
```

也可以每次只构建一个平台。脚本准备平台构建，并启用 `ROUTELOOM_APPLE_STORE=true`，使用由 Apple 管理的更新渠道。它将 `ROUTELOOM_PRIVACY_URL` 嵌入使用前的数据声明；签名归档必须提供该网址。缺少该网址时，客户端无法从应用内发起新的连接。用于编译检查的未签名 CI 归档可以不提供网址，但受到相同的应用内限制。这项界面限制不会停止操作系统管理的已有或按需 VPN 会话；迁移测试需要单独验证这些情况。

脚本会生成本地 `.xcarchive`，打印路径并打开归档，供发布者在 Xcode 中继续分发。脚本不会上传构建、提交审核、导出签名凭据或发布下载链接。遇到构建或签名错误请立即停止，不要误发以前成功生成的旧归档。

在 Xcode Organizer 中核对平台、版本、构建号、团队与 Bundle ID。在界面提供时使用 **Validate App**，再选择 **Distribute App → TestFlight & App Store**。按 Xcode 提示分别为两个平台完成签名及上传。需要拥有上传权限的 App Store Connect 角色。等待 Apple 处理构建，解决报告的问题后，再选择该构建进行测试。[上传构建](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds)

## 从 TestFlight 到客户正式安装

1. 先在实体 iPhone/iPad 和 Mac 上内部测试：安装、导入独立配置、授权 VPN、连接、查看流量并运行连接测试。验证自动/代理/直连模式、局域网访问、重连、睡眠唤醒及网络切换。测试覆盖安装，确认已保存配置仍可用。
2. 提供测试审核信息，将构建加入外部测试组。首个外部测试构建需经过 Apple 审核。批准后创建并验证公开 TestFlight 链接。TestFlight 最多支持 10,000 名外部测试者，每个构建有效期最多为 90 天，因此是测试渠道，不是永久正式发布。[TestFlight](https://developer.apple.com/testflight/)、[TestFlight 概述](https://developer.apple.com/help/app-store-connect/test-a-beta-version/testflight-overview)
3. 仅在网站添加已验证的 TestFlight 链接。客户安装 Apple 的 TestFlight 应用，打开链接并安装 RouteLoom，然后导入自己的私有设备配置并接受系统 VPN 提示。此流程无需 Xcode 或开发者模式。
4. 测试通过后，分别提交各平台的 App Review。审核批准并正式发布后，才添加真实商店链接。客户随后通过相应 App Store 安装和更新。审核结果与时间由 Apple 决定。

## Mac 网站 DMG 下载需要单独实现

目前 Mac 的 VPN provider 是应用扩展（`.appex`）。Apple 的部署表将这种打包方式限制在 App Store 分发。要通过网站/GitHub 提供 Developer ID 签名安装包，需要将 provider 迁移为**系统扩展**，实现激活和更新生命周期，正确配置权限及描述文件，完成 Developer ID 签名和公证。仅将当前开发版套入 DMG，并不能获得这种分发能力。App Store 归档流程没有实现这项迁移。[Apple TN3134](https://developer.apple.com/documentation/technotes/tn3134-network-extension-provider-deployment)

已有个人签名开发版请使用 [Apple 升级指南](upgrade-apple.zh-CN.md)。在签名、身份迁移和升级行为验证完成前，它们与经批准的客户发布渠道仍是不同安装路径。
