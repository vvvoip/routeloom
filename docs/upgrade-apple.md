# Upgrade an existing personally signed Apple installation

[简体中文](upgrade-apple.zh-CN.md)

This guide upgrades a working 0.3 checkout to the 0.4 application sources while retaining the locally signed Xcode projects. Keep the same Apple team and bundle IDs for Runner and PacketTunnel. Do not uninstall the apps or delete saved profiles, Keychain entries, or the encrypted DNS mapping cache.

1. Disconnect RouteLoom on the Mac and iPhone. If protection keeps it blocked, use **Restore direct internet**. Quit the apps and stop both Xcode Run sessions.
2. In Mac Terminal, run the entire block from anywhere inside your RouteLoom Git checkout, including `apps/client`. It finds the repository root automatically, backs up tracked local edits and replaces only shared application/build sources. The `ios` and `macos` Xcode projects, entitlements and signing settings stay in place. The Git checkout remains on its original branch/tag; the app version becomes **0.4.0+4**.

```sh
bash <<'ROUTELOOM_UPDATE'
set -euo pipefail
routeloom_root="$(git rev-parse --show-toplevel)"
cd "$routeloom_root"
test -f Cargo.toml
test -f apps/client/pubspec.yaml
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

Do not skip the Rust builds: the Apple dashboard calls new native functions. If a command fails, stop and share that error (without your private profile).

If an earlier attempt reported `pathspec 'Cargo.toml' did not match`, it likely ran from a subdirectory. Run this corrected block in full. The `CRLF will be replaced by LF` warning is informational and is not the cause of that error.

3. In the **macOS** workspace, check **PacketTunnel → Signing & Capabilities → App Sandbox**: both **Incoming Connections (Server)** and **Outgoing Connections (Client)** must be checked. This is required on the extension, even when already enabled on Runner. Keep your existing team and bundle IDs. Choose **Product → Scheme → Edit Scheme → Run → Info → Build Configuration: Release**, uncheck **Debug executable**, select **My Mac**, then **⌘R**.
4. In the **iOS** workspace, select your connected, unlocked iPhone. Keep the same team and IDs on **Runner and PacketTunnel**. Choose the same Release Run configuration, then **⌘R**. Xcode installs over the existing app. Do not delete the old app first.

After testing the Mac build, if you normally launch a separate copy from `/Applications`, quit it and replace it in Finder with the newly built `apps/client/build/macos/Build/Products/Release/routeloom.app`. Use the same app name; do not accidentally relaunch the old copy. Launch from Applications after replacing it.

## Apply and test the routing preset

On each device:

1. Open **Settings**, select your existing profile, and choose **China and LAN direct** while disconnected. Confirm replacement if you had custom rules. It preserves server credentials and selects Auto.
2. Connect. The middle tab is now **Traffic**. Open `www.baidu.com` and `www.google.com`, then search their domains in Traffic. Baidu should show **Direct** and Google **Proxy**; the matched rule appears on the connection row. CDN requests can have their own routes.
3. Choose **Test connection**. With this preset, `api.ipify.org` uses Proxy and should show your server's public exit IP. This checks one HTTPS request, not every application.
4. To compare modes, disconnect, choose **Proxy**, reconnect and make fresh requests: captured flows should use Proxy. Repeat with **Direct**: captured flows and DNS use the underlying connection; the exit IP can differ and some sites may be unreachable. Return to **Auto** afterwards.

Existing browser connections may need a new tab or browser restart to produce fresh records. The panel shows captured TCP/UDP only. Existing more-specific OS routes can bypass capture. The preset is a curated domain list, not a complete geographic database; app-managed encrypted DNS and literal IPs follow IP/default rules. Auto DNS still uses the server. See [full behavior and limits](client-features.md).
