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
if [ -f "${CARGO_HOME:-$HOME/.cargo}/env" ]; then
  source "${CARGO_HOME:-$HOME/.cargo}/env"
fi
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
if command -v brew >/dev/null 2>&1; then
  routeloom_rustup_prefix="$(brew --prefix rustup 2>/dev/null || true)"
  if [ -n "$routeloom_rustup_prefix" ] && [ -x "$routeloom_rustup_prefix/bin/rustup" ]; then
    export PATH="$routeloom_rustup_prefix/bin:$PATH"
  fi
fi
command -v rustup
command -v cargo
cargo --version
git fetch origin tag v0.4.0-alpha.1
mkdir -p .local
backup=".local/before-0.4-$(date +%Y%m%d-%H%M%S).patch"
git diff --binary HEAD > "$backup"
git restore --source=v0.4.0-alpha.1 --worktree -- \
  Cargo.toml Cargo.lock rust-toolchain.toml crates \
  apps/client/apple apps/client/lib apps/client/assets apps/client/test \
  apps/client/pubspec.yaml apps/client/pubspec.lock \
  scripts/build-native.sh scripts/collect-licenses.mjs scripts/bundle-client-licenses.mjs
for platform in macos ios; do
  /usr/libexec/PlistBuddy -c 'Set :CFBundleShortVersionString 0.4.0' "apps/client/$platform/PacketTunnel/Info.plist"
  /usr/libexec/PlistBuddy -c 'Set :CFBundleVersion 4' "apps/client/$platform/PacketTunnel/Info.plist"
done
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

The shell setup file `.cargo/env` is optional. The block supports the normal/custom Cargo directory and Homebrew's `rustup` executable directory, then checks that `rustup` and `cargo` are available before updating sources. A missing setup file alone does not mean Rust needs reinstalling. If either tool cannot be found, stop and share the output of `command -v brew rustup cargo rustc` so the existing installation can be located. See [Homebrew's Rustup instructions](https://formulae.brew.sh/formula/rustup).

3. In the **macOS** workspace, check **PacketTunnel → Signing & Capabilities → App Sandbox**: both **Incoming Connections (Server)** and **Outgoing Connections (Client)** must be checked. This is required on the extension, even when already enabled on Runner. Keep your existing team and bundle IDs. Choose **Product → Scheme → Edit Scheme → Run → Info → Build Configuration: Release**, uncheck **Debug executable**, select **My Mac**, then **⌘R**.
4. In the **iOS** workspace, select your connected, unlocked iPhone. Keep the same team and IDs on **Runner and PacketTunnel**. Choose the same Release Run configuration, then **⌘R**. Xcode installs over the existing app. Do not delete the old app first.

After testing the Mac build, if you normally launch a separate copy from `/Applications`, quit it and replace it in Finder with the newly built `apps/client/build/macos/Build/Products/Release/routeloom.app`. Use the same app name; do not accidentally relaunch the old copy. Launch from Applications after replacing it.

### New interface, but Traffic is unavailable

The app and PacketTunnel are separate binaries. An older registered extension or an incomplete native build can leave the updated interface without monitoring. The original 0.4 signing kit also retained the extension's old `0.1.0`/`1` metadata; the commands above correct those two fields while preserving your Keychain group. The version label alone does not establish which code is running.

First restore direct internet, stop Xcode's Run session, and quit RouteLoom. Use **Product → Clean Build Folder** in the correct macOS workspace, then rebuild the Runner scheme in Release. Check that PacketTunnel is built and embedded successfully; do not run an older successful build after a build error. Test in Proxy mode before applying Auto rules. If macOS still uses a different app copy, inspect it with `pgrep -fl PacketTunnel` and `pluginkit -m -A -D -v -i YOUR_BUNDLE_ID.PacketTunnel`. Do not paste raw extended VPN status, which may contain private profile details from older builds.

## Apply and test the routing preset

On each device:

1. Open **Settings**, select your existing profile, and choose **China and LAN direct** while disconnected. Confirm replacement if you had custom rules. It preserves server credentials and selects Auto.
2. Connect. The middle tab is now **Traffic**. Open `www.baidu.com` and `www.google.com`, then search their domains in Traffic. Baidu should show **Direct** and Google **Proxy**; the matched rule appears on the connection row. CDN requests can have their own routes.
3. Choose **Test connection**. With this preset, `api.ipify.org` uses Proxy and should show your server's public exit IP. This checks one HTTPS request, not every application.
4. To compare modes, disconnect, choose **Proxy**, reconnect and make fresh requests: captured flows should use Proxy. Repeat with **Direct**: captured flows and DNS use the underlying connection; the exit IP can differ and some sites may be unreachable. Return to **Auto** afterwards.

Existing browser connections may need a new tab or browser restart to produce fresh records. The panel shows captured TCP/UDP only. Existing more-specific OS routes can bypass capture. The preset is a curated domain list, not a complete geographic database; app-managed encrypted DNS and literal IPs follow IP/default rules. Auto DNS still uses the server. See [full behavior and limits](client-features.md).
