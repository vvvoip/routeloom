# Apple customer distribution

English · [简体中文](apple-customer-distribution.zh-CN.md)

RouteLoom's customer distribution path is **TestFlight for beta testing, followed by the App Store for iPhone/iPad and the Mac App Store for Mac**. Customers use an installation link and do not need Xcode, Rust, Flutter, their own developer account, or Developer Mode. The publisher builds and signs the apps once per release. [Apple distribution methods](https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases)

## Current status

The GitHub Apple ZIP is still a source signing kit. No public RouteLoom TestFlight, App Store, Mac App Store or signed DMG download is published by this change. A locally produced archive is not a customer installer, and an unsigned CI build does not prove distribution signing or physical-device behavior.

Development builds have connected on the owner's Mac and iPhone. The updated Mac traffic dashboard and connection test still need physical-device confirmation; sleep/wake, network changes, updates and installation on another customer's device also need validation before public distribution. Keep the existing personal testing installation while preparing the publisher's release.

## Publisher account and app setup

1. Confirm that the publishing team is an **Apple Developer Program organization**. Apple's VPN guideline requires organization enrollment; a working personal development certificate alone does not establish eligibility. If the account is individual, resolve enrollment with Apple before promising public VPN distribution. [App Review guideline 5.4](https://developer.apple.com/app-store/review/guidelines/#vpn-apps)
2. Choose permanent publisher-owned bundle identifiers and create the matching App Store Connect app record/platform entries. Runner and its `.PacketTunnel` extension must use the same team and matching Network Extensions and Keychain entitlements. Check signing and provisioning for **both targets on both platforms**. Use the appropriate Apple distribution signing assets in Xcode's distribution flow. [Configure Network Extensions](https://developer.apple.com/documentation/xcode/configuring-network-extensions/)
3. Preserve the existing team, bundle IDs and Keychain groups when updating an existing installation. Moving from a personal team to an organization may create a separate app identity; do not assume the new app can read the old app's saved profiles. Keep private profile backups available for a deliberate migration.
4. Create real English and Simplified Chinese listing text, screenshots, support details, age-rating answers and privacy information. Verify the bundled RouteLoom icons with the publisher and prepare the required store screenshots. The publisher must supply and approve its identity, contact information, data practices and privacy-policy URL; repository examples are not a completed policy. Producing an archive alone does not make the app ready for submission.

Signing credentials stay with the publisher. Do not paste Apple passwords, signing private keys, App Store Connect API private keys or private server profiles into a chat, public issue or GitHub repository. Local Xcode signing avoids exporting those credentials to CI.

## Privacy and review preparation

The in-app disclosure and published privacy policy must describe the actual release, including the selected server's role, credentials stored on the device, connection metadata displayed in memory, encrypted DNS mapping storage and any optional outbound diagnostic requests. Describe the publisher's server operations accurately; do not claim that third-party servers have identical practices. Apple requires the VPN data declaration before service use and specific commitments about VPN data handling. The publisher must review the wording against its real practices. [VPN privacy requirements](https://developer.apple.com/app-store/review/guidelines/#vpn-apps)

Complete App Store Connect's encryption questions for the actual implementation and intended distribution regions. RouteLoom includes Rust TLS/cryptography libraries and an encrypted local DNS cache; it is not merely an app calling the operating system's HTTPS API. Do not insert `ITSAppUsesNonExemptEncryption = false` without determining that the claim is correct. Any required documentation or classification must come from the publisher's review of its obligations. [Apple export-compliance guidance](https://developer.apple.com/help/app-store-connect/manage-app-information/overview-of-export-compliance/)

Choose release territories and supply any license information Apple requires for those territories. This guide does not determine local licensing eligibility. Give App Review a dedicated, working review profile and instructions through the private review fields. Use a separate revocable test credential, keep the test server available during review, and never include that credential in release archives or public documentation. [Apple review preparation](https://developer.apple.com/app-store/review/)

## Update an existing personalized checkout

Use this section if the Mac checkout is still on an older tag or was updated by selectively copying files. **This workflow is available only after the customer-distribution changes have merged into `main`.** The first check below stops if the archive script is not present on the fetched branch. A clean checkout of current `main` does not need this update block.

Disconnect RouteLoom on the Mac and iPhone; use **Restore direct internet** if protection prevents disconnection. Quit the apps and stop Xcode Run sessions. Run the whole block from any directory inside the repository. It backs up tracked edits, updates the shared client/native/build sources and committed icons, and changes only the two version fields in each extension's Info.plist. Your personalized Xcode projects, signing settings, entitlements, Keychain groups and saved profiles remain in place.

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

The checkout may still display its original tag and a dirty working tree; this is expected because the signed project was preserved. Keep the backup local. If a command fails, stop and resolve that error before building. The archive script below rebuilds the Rust library and app from the updated sources; do not reuse an older app bundle.

## Build archives on the publisher's Mac

For the Mac target, set **Runner → General → App Category** in Xcode (Utilities is appropriate for this client) before preparing the signed archive. This writes `LSApplicationCategoryType` to the personalized app configuration. Unsigned CI can compile without a category, but that does not complete the Mac App Store metadata requirements.

Use the updated source checkout with Xcode, Flutter, Rust/rustup and Node.js installed. Sign in to Xcode using the publishing team and configure Runner and PacketTunnel first. Publish the publisher-approved privacy policy at a real, publicly accessible HTTPS URL. Replace the placeholder below with that URL; a private draft or example policy is not sufficient. The build commands retain those existing project identities. From anywhere inside the repository:

```sh
cd "$(git rev-parse --show-toplevel)"
export ROUTELOOM_PRIVACY_URL='<published HTTPS privacy-policy URL>'
bash scripts/archive-apple-store.sh ios
bash scripts/archive-apple-store.sh macos
```

Run one platform at a time if preferred. The script prepares the platform build and enables `ROUTELOOM_APPLE_STORE=true` for the Apple-managed update channel. It embeds `ROUTELOOM_PRIVACY_URL` for the pre-use disclosure and requires that URL for a signed archive. Without that URL, the client cannot start a new connection from the app. Unsigned CI archives may omit it for compilation checks and have the same app-level restriction. This UI gate does not stop an existing or on-demand VPN session managed by the operating system; migration testing must cover those sessions separately.

The script creates a local `.xcarchive`, prints its location and opens it for the publisher's Xcode distribution workflow. It does not upload a build, submit a review, export signing secrets or publish a download link. Stop at the first build/signing error; do not distribute a previous successful archive by mistake.

In Xcode Organizer, inspect the archive's platform, version, build number, team and bundle identifiers. Use **Validate App** where offered, then **Distribute App → TestFlight & App Store**. Follow Xcode's signing and upload steps for each platform. An App Store Connect role with upload permission is required. Wait for Apple's build processing and resolve any reported issues before selecting the build for testing. [Uploading builds](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds)

## TestFlight, then customer release

1. Test internally on a physical iPhone/iPad and Mac: install, import a dedicated profile, grant VPN permission, connect, inspect traffic and run the connection test. Verify Auto, Proxy and Direct, local-network access, reconnect, sleep/wake and network changes. Test an update over an existing installation and verify its saved profile remains available.
2. Supply beta review information and add the build to an external testing group. The first external build requires Apple review. After approval, create and verify the public TestFlight link. TestFlight supports up to 10,000 external testers and each build lasts up to 90 days, so it is a beta channel rather than a permanent customer release. [TestFlight](https://developer.apple.com/testflight/), [TestFlight overview](https://developer.apple.com/help/app-store-connect/test-a-beta-version/testflight-overview)
3. Add only the verified TestFlight link to the website. Customers install Apple's TestFlight app, open the link and install RouteLoom. They import their own private device profile and accept the OS VPN prompt. Xcode and Developer Mode are unnecessary for this path.
4. After beta validation, submit each platform for App Review. Publish its actual store link only after approval and release. Customers then install and receive updates through the relevant App Store. Approval and timing are controlled by Apple.

## A downloadable Mac DMG is a separate implementation

The current Mac VPN provider is an app extension (`.appex`). Apple's deployment table restricts that packaging to App Store distribution. A website/GitHub installer using Developer ID requires the provider to be packaged as a **system extension**, including its activation/update lifecycle, appropriate entitlements and provisioning, Developer ID signing and notarization. Merely wrapping the current development app in a DMG does not create that distribution path. That migration is not implemented by the App Store archive workflow. [Apple TN3134](https://developer.apple.com/documentation/technotes/tn3134-network-extension-provider-deployment)

For existing personally signed development builds, use the [Apple upgrade guide](upgrade-apple.md). Those installations remain separate from an approved customer distribution channel until signing, identity migration and update behavior have been validated.
