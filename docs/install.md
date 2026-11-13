# Install the RouteLoom alpha
English · [简体中文](install.zh-CN.md)

Download assets from [GitHub Releases](https://github.com/vvvoip/routeloom/releases). Use the files attached under **Assets**, not GitHub's automatic source-code download. A private RouteLoom server profile is required; downloads contain no shared credentials.

## Windows 10/11, x64
1. Download routeloom-windows-x64.zip and extract **all** files into a new folder.
2. Double-click Start-RouteLoom.cmd and accept the Windows administrator prompt. The alpha GUI and its helper currently run elevated. If the prompt is cancelled, no VPN starts.
3. Open your windows.profile.json privately, copy its complete JSON text, and select **Import profile from clipboard** in RouteLoom.
4. Choose **Connect**. Wait for **Connected**, then test your browser or terminal. Choose **Disconnect** before closing.

The ZIP includes the Rust helper, signed Wintun DLL and Microsoft C++ runtime. Do not run the EXE from inside the ZIP or move it away from its data/DLL files. RouteLoom itself is not yet code-signed and may trigger SmartScreen. Verify the release checksum; do not disable Windows security globally. Production service-based elevation and signed installers are still pending.

## Ubuntu 24.04+ desktop, amd64
Download routeloom-linux-amd64.deb, then install from its folder:

```sh
sudo apt install ./routeloom-linux-amd64.deb
```

Launch RouteLoom from the application menu. Import your linux.profile.json, choose Connect and authenticate in the desktop permission dialog. Only the network helper runs as root. A polkit authentication agent and active systemd-resolved service are required. There is no passwordless/sudoers rule.

The package requires glibc 2.39 and is not a Debian 12 desktop build. Other Linux distributions/architectures are not covered by this package. The separate server can run on Debian 12.

To remove: choose **Restore direct internet** first, then `sudo apt remove routeloom`. If the desktop has no permission agent, use the CLI from a terminal: `sudo /usr/lib/routeloom/routeloom-vpn connect /absolute/path/linux.profile.json`; Ctrl-C disconnects.

## Android 10+
Download routeloom-android.apk. Open it on your Android device and allow installation from that download app if prompted. Open RouteLoom, copy/import your android.profile.json, choose Connect and accept Android's VPN permission. The foreground notification includes Disconnect.

This build uses the permanent RouteLoom signing key. The old 0.1 preview used a different key: save your private profile, uninstall that preview once, install 0.2 and reimport. It is not a Play Store release.

## macOS and iOS/iPadOS
Already running a personally signed Apple build? Follow the [0.4 upgrade guide](upgrade-apple.md) to preserve your signing projects and saved profiles. Settings displays the installed application version.

The Apple download is a **source signing kit**, not an installable DMG/IPA. Apple VPN entitlements require your signing/provisioning setup. Do not disable SIP/Gatekeeper as a substitute for signing.

On your Mac, install Xcode, Flutter 3.50.0, Node.js and Rust/rustup, sign in to Xcode with your Apple Developer account, unzip the signing kit and run:

```sh
cd routeloom
bash scripts/prepare-apple.sh YOURTEAMID com.yourname.routeloom
```

Replace YOURTEAMID with your 10-character Apple Team ID and use a unique reverse-domain bundle identifier. The script configures both apps and PacketTunnel extensions, prepares Flutter dependencies and opens both workspaces. Select your Mac or connected iPhone/iPad in Xcode. Check **Signing & Capabilities** for **Runner and PacketTunnel**, including Network Extensions, then Run. Import the matching profile and grant VPN permission.

This prepares personal development testing. TestFlight/App Store and signed macOS distribution are separate work and are not published yet. See [Apple VPN development](vpn-development.md#macos-and-iosipados).

## Connect and verify
Each device needs a unique profile. Keep tokens and certificate material out of GitHub, screenshots and public websites. Profiles and preferences are saved in protected system storage. An unavailable keyring is reported explicitly; see [client settings](client-features.md). The app supports English and Simplified Chinese.

Verify a public IP check reports your server's exit IP, try a browser and terminal, then disconnect and verify normal access returns. Literal IPv6 destinations require an IPv6 upstream. Domain flows can use the server’s IPv4 exit.

Version 0.2 includes domain/PAC routing, reconnect, saved profiles, persistent protection controls and signed update checks. Read [settings, platform limits and recovery](client-features.md) before enabling protection. Disconnect does not remove a persistent guard: choose Restore direct internet before uninstalling. Android requires system lockdown settings; personally signed Apple updates still require Xcode. Physical mobile-device validation remains pending.

## Checksums
Compare the downloaded asset against SHA256SUMS.txt from the same release:

```powershell
Get-FileHash .\routeloom-windows-x64.zip -Algorithm SHA256
```
```sh
sha256sum routeloom-linux-amd64.deb
# macOS:
shasum -a 256 routeloom-apple-signing-kit.zip
```
