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

To remove: disconnect first, then `sudo apt remove routeloom`. If the desktop has no permission agent, use the CLI from a terminal: `sudo /usr/lib/routeloom/routeloom-vpn connect /absolute/path/linux.profile.json`; Ctrl-C disconnects.

## Android 10+
Download routeloom-android-preview.apk. Open it on your Android device and allow installation from that download app if prompted. Open RouteLoom, copy/import your android.profile.json, choose Connect and accept Android's VPN permission. The foreground notification includes Disconnect.

This is a release-mode build signed with a preview key, not a Play Store release. Signing keys can change between alpha builds; uninstall the old preview if Android refuses an update. Keep your private profile separately so you can import it again.

## macOS and iOS/iPadOS
The Apple download is a **source signing kit**, not an installable DMG/IPA. Apple VPN entitlements require your signing/provisioning setup. Do not disable SIP/Gatekeeper as a substitute for signing.

On your Mac, install Xcode and Flutter 3.50.0, sign in to Xcode with your Apple Developer account, unzip the signing kit and run:

```sh
cd routeloom
bash scripts/prepare-apple.sh YOURTEAMID com.yourname.routeloom
```

Replace YOURTEAMID with your 10-character Apple Team ID and use a unique reverse-domain bundle identifier. The script configures both apps and PacketTunnel extensions, prepares Flutter dependencies and opens both workspaces. Select your Mac or connected iPhone/iPad in Xcode. Check **Signing & Capabilities** for **Runner and PacketTunnel**, including Network Extensions, then Run. Import the matching profile and grant VPN permission.

This prepares personal development testing. TestFlight/App Store and signed macOS distribution are separate work and are not published yet. See [Apple VPN development](vpn-development.md#macos-and-iosipados).

## Connect and verify
Each device needs a unique profile. Keep tokens and certificate material out of GitHub, screenshots and public websites. Profiles stay in memory for the current app session; reopen/import after restarting the app. The app supports English and Simplified Chinese.

Verify a public IP check reports your server's exit IP, try a browser and terminal, then disconnect and verify normal access returns. If the server has no IPv6 upstream, IPv6 is captured but cannot reach the Internet.

This alpha does not yet provide a persistent kill switch, automatic reconnect, saved profiles, automatic updates, or domain/PAC rules applied to VPN traffic. Disconnect/failure releases capture, and existing more-specific system routes may remain direct. Avoid treating this as a finished privacy product.

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
