# Architecture

Status: VPN developer milestone / pre-alpha. [RL/1 specification](rl1.md) and [setup guide](vpn-development.md).

## Implemented boundaries

- **Policy core (Rust):** deterministic rule validation, first-match decisions, reason lines, and lossless PAC export. It remains independent of network access.
- **Packet transport (Rust):** TLS 1.3, CA/hostname verification, device authentication, bounded frames, IPv4/IPv6 validation and lease checking.
- **Desktop capture:** Rust Wintun/TUN helper on Windows/Linux. Explicit routes exclude the numeric tunnel endpoint. Owned routes/DNS are reversed on normal exit. The Flutter bridge passes the profile over child stdin, not command-line arguments.
- **Android:** VpnService with consent and foreground lifecycle. Its TLS socket is protected before capture. Native packet forwarding operates independently of Flutter.
- **Apple:** macOS/iOS share a Network Extension packet provider and native TLS implementation. Both Xcode projects embed the extension. Signing, entitlement and physical-device validation are external gates.
- **Server:** routeloom-vpn serve creates a Linux TUN, authenticates each device, checks source leases and egress policy, and dispatches return traffic using bounded queues. Kernel forwarding/NAT is deployment configuration. The older routeloom-server health command remains a separate loopback-only scaffold.
- **Interface:** Flutter with English and Simplified Chinese, session-only profile import, Connect/Disconnect and native status reporting.
- **Website:** static bilingual GitHub Pages site, independent of the tunnel host.

## Current data path

Application → OS IP capture → RL/1 packets over authenticated TLS 1.3 → Linux server TUN → host egress firewall/routing → destination.

Both IPv4 and IPv6 carry TCP, UDP and DNS packets. IP capture routes are configurable. The existing domain/PAC rules are **not yet integrated into packet processing**; the UI and docs do not imply otherwise.

## Planned integrations

HTTP/SOCKS ingress, domain-aware flow classification, HTTP/2 CONNECT and HTTP/3 CONNECT-UDP, credential enrollment/revocation without restart, automatic reconnect, persistent kill switch, and least-privilege service installers. RL/1 is an initial developer transport with TCP-over-TCP performance limitations, not a replacement for standard cryptography.

IP geography remains approximate and dataset licenses need review. A country rule does not create an exit node in that country. LAN/private-cloud egress requires explicit server allow-listing; NAT traversal connectors are future work.

## Lifecycle and rollback

Validate profile → verify TLS → authenticate device/lease → install capture and DNS → report connected. Failures close capture and unwind owned desktop changes. Disconnect is fail-open: a persistent kill switch is not implemented. Hard kills, suspend/resume, network transitions and coexistence with other VPNs need real-device tests. No claim of universal DNS-leak prevention is made.

## Apple constraints

Use supported packet tunnels to remote servers. iOS per-app VPN requires managed devices. macOS App Store/development app-extension and Developer ID system-extension distribution differ; the latter packaging is not implemented.

References: [Apple TN3120](https://developer.apple.com/documentation/technotes/tn3120-expected-use-cases-for-network-extension-packet-tunnel-providers), [Apple TN3134](https://developer.apple.com/documentation/technotes/tn3134-network-extension-provider-deployment), [Android VPN](https://developer.android.com/develop/connectivity/vpn), [rustls](https://docs.rs/rustls/), [CONNECT-UDP](https://www.rfc-editor.org/rfc/rfc9298.html).
