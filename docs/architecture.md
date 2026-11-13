# Architecture

Status: VPN developer milestone / pre-alpha. See [RL/1](rl1.md) and [native setup](vpn-development.md) for the implemented data path and validation limits.

The original planned boundaries below remain the long-term architecture. RL/1 currently carries complete IPv4/IPv6 packets over TLS 1.3; it does not yet apply domain rules or provide HTTP CONNECT/CONNECT-UDP.

## Boundaries

- **Policy core (Rust):** pure validation and routing decisions. No network access, DNS resolution, credentials, or platform mutations. First match wins; a final action is mandatory. Match reasons retain source-line numbers.
- **CLI:** exercises the core and exports the lossless domain-only PAC subset. Configuration keys remain stable English identifiers; output labels support English and Simplified Chinese. Detailed developer errors are currently English.
- **Client (Flutter):** shared bilingual interface. Native adapters implement an explicit lifecycle. Pending adapters throw on connect; they never report a fake connected state.
- **OS adapter:** Windows Wintun/service; Linux TUN/service; Android VpnService; Apple Network Extension. Desktop core communication will use authenticated local IPC. Mobile embedding will use a narrow native bridge. Neither bridge is implemented yet.
- **Server:** current loopback-only health scaffold. Planned authenticated TCP/UDP egress, per-device credentials and revocation, enrollment, limits, and an admin CLI. The health scaffold is not a production HTTP stack.
- **Website:** static HTML/CSS/JS with complete English and Chinese pages. No tracking, external fonts, or secrets. GitHub Pages is separate from the future tunnel server.

## Planned data path

Application → OS capture or local HTTP/SOCKS endpoint → DNS/flow metadata → routing policy → direct / reject / encrypted tunnel → server authorization → permitted destination.

TCP transport: TLS 1.3 and HTTP/2 CONNECT. UDP transport: CONNECT-UDP over HTTP/3, subject to feasibility and interoperability testing. Any custom extension requires an ADR and public specification. Do not invent cryptography.

## DNS and bypasses

Caller-supplied IPs are evaluated without implicit DNS. Future DNS integration must define resolver selection, bootstrap resolution, IPv6 behavior, and encrypted-DNS limitations. Hostname rules cannot reliably classify opaque HTTPS paths. IP geography is approximate and dataset licenses need review. A country rule does not create an exit node in that country.

Internal-network access means explicit LAN or private-cloud routes. Remote networks behind NAT require a future connector. Protect server metadata and management endpoints; no private egress without explicit authorization.

## Lifecycle and rollback

Prepare credentials → authenticate transport → apply DNS/routes → confirm ready. On failure, unwind owned changes and emit failure. Kill-switch behavior must preserve only configured direct exceptions. Reconnect, suspend/resume, network changes and coexistence with other VPNs require real-device tests.

## Apple constraints

Use supported Network Extension APIs and confirm entitlement/distribution requirements early. Do not use packet tunnels as arbitrary local content filters. iOS per-app VPN requires managed devices. Device-wide routing features may differ from Windows/Android.

Primary references: [Apple TN3120](https://developer.apple.com/documentation/technotes/tn3120-expected-use-cases-for-network-extension-packet-tunnel-providers), [Apple TN3134](https://developer.apple.com/documentation/technotes/tn3134-network-extension-provider-deployment), [Android VPN](https://developer.android.com/develop/connectivity/vpn), [HTTP/2](https://www.rfc-editor.org/rfc/rfc9113.html), [CONNECT-UDP](https://www.rfc-editor.org/rfc/rfc9298.html).
