# VPN developer milestone

English · [简体中文](vpn-development.zh-CN.md)

This build contains a TLS 1.3 packet server, Windows/Linux TUN helper, Android VpnService, and macOS/iOS PacketTunnelProvider. It is a developer build. Passing unsigned compilation does not prove signed VPN operation on a real device.

For downloadable alpha packages, start with the [installation guide](install.md).

## Local server and profiles

Build with Rust 1.100: `cargo build --release -p routeloom-vpn --locked`.
On Linux, install OpenSSL 3, iproute2, and a kernel with /dev/net/tun.

```sh
node scripts/create-dev-profile.mjs .local/demo SERVER_IP:9443 tunnel.example
sudo target/release/routeloom-vpn serve .local/demo/server.json
```

The generator creates a 30-day development certificate, a private key, a device profile and a server configuration. Transfer the profile privately. Keep the output out of Git. The server stores only the token hash. Add a device with a unique token, IPv4 lease in 10.77.0.2–254 and IPv6 lease in fd77::2–ffff. Production certificate lifecycle and enrollment are not automated.

The server creates rl-server (10.77.0.1/24 and fd77::1/64). For Internet egress, configure Linux forwarding and firewall/NAT on a dedicated test host; see [deployment](../deploy/vpn.md). The repository's CI test uses isolated namespaces and requires no cloud host.

## Windows and Linux

The Rust helper creates Wintun/TUN, excludes the numeric server endpoint, and applies profile routes/DNS. The shared RL/2 engine classifies TCP/UDP and retains capture during reconnect. Disconnect removes owned routes; opt-in persistent WFP/nftables protection is removed only by explicit unlock. Existing more-specific OS routes may bypass capture unless blocked by the guard. See [client settings and recovery](client-features.md).

- Windows: administrator privileges, signed Wintun DLL beside routeloom-vpn.exe. The download bundles the signed Wintun runtime from the pinned dependency and its license. Run `routeloom-vpn connect device.profile.json` in an administrator terminal.
- Linux: root, iproute2 and systemd-resolved/resolvectl. Run `sudo routeloom-vpn connect device.profile.json`. Systems using another resolver are not supported by the current DNS integration.
- Flutter desktop: place the helper beside the app, or set ROUTELOOM_VPN_BINARY to an absolute helper path. On Linux, the GUI invokes the helper through pkexec and a desktop authentication agent. On Windows, the preview launcher elevates the GUI and the helper inherits its privileges. Start with the CLI for development. A least-privilege installed service and elevation UI remain packaging work. Do not distribute a privileged GUI as the production design.
- Disconnect with Ctrl-C. The Flutter bridge uses a private child-process stdin channel for profiles and graceful shutdown; credentials never appear in command-line arguments.

## Android

Android 10/API 29+ is required. From the repository root run `bash scripts/build-native.sh android`, collect/bundle licenses as in CI, then `flutter build apk --debug` in apps/client. Use JDK 21 for the current Android lint tooling. NativeEngine JNI runs the shared Rust engine and protects its sockets with VpnService.protect. AndroidKeyStore stores the background profile. Enable system Always-on VPN and Block connections without VPN for persistent protection.

## macOS and iOS/iPadOS

Open the generated Runner Xcode workspace on your Mac. Both projects include a PacketTunnel target and the same shared Swift sources. Set your Apple development team on **Runner and PacketTunnel**, register unique app/extension identifiers, and enable the Network Extensions capability for both provisioning profiles. Keep extension identifier equal to the app identifier plus `.PacketTunnel`. The checked-in identifiers are provisional.

Build and run on your Mac and physical iPhone/iPad, then import the profile and grant VPN permission. Unsigned CI builds only verify compilation and embedding. App Store/TestFlight/Developer ID distribution and entitlement approval are separate. The macOS app-extension path must use a distribution method supported by Apple; a Developer ID system-extension variant is not implemented here.

Profiles are passed as start options and retained in the app/extension shared Keychain for OS restart; secret profiles are not saved to VPN preferences. Build native libraries with scripts/prepare-apple.sh before opening Xcode. The OS may retain the non-secret VPN configuration. iOS per-app routing still requires managed devices.

## Verification

`cargo test --workspace --locked` checks framing, packet structure, TLS trust and authentication, leases and endpoint route exclusions. `sudo bash scripts/test-vpn-linux.sh` exercises real IPv4/IPv6 packets through two network namespaces and verifies cleanup. Its resolver command is stubbed; it does not establish DNS-leak protection. Flutter tests cover profile validation and the connection controls.

Real-device gates: all five OSes, grant/deny/revoke permission, wrong cert/token, concurrent device rejection, browser/terminal TCP, UDP and DNS, IPv6, server loss, disconnect cleanup, sleep/wake, Wi-Fi/mobile changes, and coexistence with another VPN. Record results before publishing signed installers. There is no blanket “all traffic protected” claim.
