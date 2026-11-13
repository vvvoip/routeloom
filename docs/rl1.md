# RL/1 development packet tunnel

Historical protocol documentation for 0.1 clients. Version 0.2 uses [RL/2](rl2.md). The server preserves this legacy path; limitations below describe older clients.

RL/1 is a versioned IP framing protocol carried over **TLS 1.3 only**, ALPN `routeloom/1`. It introduces no encryption algorithm. Rust uses rustls/ring; Android uses the platform TLS stack; Apple uses Network.framework and Security.framework. The configured CA and TLS hostname must validate before credentials are sent. No insecure certificate option, TLS 1.2 fallback, or 0-RTT is offered.

Each frame is a 4-byte unsigned big-endian length (including the kind byte), one kind byte, then the payload. Length is 1–4096; packet payloads are at most 1280 bytes. Readers reject truncation, invalid lengths, and unexpected state transitions. IP packets have no Ethernet or TUN PI header.

| Kind | Direction | Payload |
| --- | --- | --- |
| 1 Hello | client → server, first | UTF-8 JSON: version=1, device, token |
| 2 Welcome | server → client, after authentication | JSON: version=1, ipv4, ipv6, mtu=1280 |
| 3 Packet | either, after Welcome | Exactly one IPv4 or IPv6 packet |
| 4 Ping | client → server | empty |
| 5 Pong | server → client | empty |

Profiles pin an endpoint IP separately from the certificate hostname, preventing bootstrap DNS and route loops. Device tokens are 32 random bytes encoded as 64 lowercase hex characters. The server stores SHA-256 hashes of that ASCII encoding and compares them in constant time. Profile and server lease addresses must agree. Duplicate simultaneous sessions for one device are rejected. Device removal/rotation requires a server restart in this milestone; restart closes existing sessions.

The server checks the authenticated source address on every packet. Clients check destination addresses. Server defaults deny private, loopback, link-local, multicast and selected special-use egress; explicit `allow_private` CIDRs permit internal networks. This is not an application-layer firewall: operators must also apply host firewall restrictions to management and metadata endpoints.

The server uses bounded per-device queues (64 packets), a 256-handshake/session semaphore, a 10-second authentication deadline and 70-second read idle deadlines. Clients send keepalives. Congested return queues drop packets. A session failure removes its routes and closes capture; this is **fail-open after disconnect**, not a persistent kill switch.

## Scope and tradeoffs

This milestone carries both IPv4 and IPv6, including TCP, UDP and DNS IP packets. It is an initial developer VPN, not a production release. TCP-over-TCP head-of-line blocking makes lossy connections slower. HTTP CONNECT, CONNECT-UDP/QUIC, domain-aware policy integration, automatic enrollment, automatic reconnect, a persistent kill switch and production service packaging remain future milestones. Rules/PAC CLI behavior is unchanged and is not silently presented as active packet filtering.

Windows/Linux use the Rust helper. Android uses VpnService and native TLS. macOS/iOS share a PacketTunnelProvider using supported Network Extension APIs. Native mobile providers continue independently of the Flutter UI. Profiles are session-only; restarting a provider requires importing/starting again.

Protocol changes require a new version/ALPN and shared conformance tests. Do not replace standard TLS cryptography.
