# RL/2 flow transport

RL/2 uses established TLS 1.3 with CA and server-name validation. ALPN is routeloom/2. It adds a separately authenticated connection per TCP/UDP flow, DNS request or heartbeat. The server still accepts routeloom/1 for older clients; new clients do not silently downgrade.

The first frame uses the RL/1 bounded envelope: a big-endian 32-bit length including one type byte. HELLO=1 contains JSON with version=2, device, token, kind (tcp, udp, resolve, ping), host and port. Unknown fields and malformed values are rejected. Device credentials are checked before egress.

TCP/UDP host must be a numeric IP and port must be nonzero. The server checks that exact IP against its public/private-destination policy before connecting; no second hostname resolution can redirect it to a private address. Private egress requires allow_private in the server configuration. Resolve accepts an ASCII DNS name with port=0; ping requires empty host and port=0.

WELCOME=2 contains JSON with version=2 and addresses (up to 16; empty except for resolve). TCP then carries an unframed byte stream. UDP uses PACKET=3 frames, one datagram per frame, at most 1,280 bytes. Ping uses PING=4/PONG=5. Resolve closes after its reply. No destination contents are logged by default.

Server limits: 512 simultaneous accepted connections, 80 authenticated RL/2 connections per device, a 10-second request/authentication deadline, 15-second TCP-connect deadline, 45-second heartbeat idle deadline, 90-second UDP idle deadlines and a 24-hour absolute TCP lifetime. TLS negotiation has its own timeout. Clients use a 15-second flow-handshake deadline and heartbeat response deadline.

## Shared client engine

All five platforms use the same Rust policy and smoltcp flow engine. Windows/Linux pass TUN packets directly; Android uses JNI and VpnService; Apple uses a static library and PacketTunnelProvider.

DNS A/AAAA interception produces stable synthetic addresses from 198.18.0.0/16 and fdfe::/112. The encrypted, locked append-only mapping journal prevents reassignment across restart/profile switches. Unknown mappings fail closed. Limits and unsupported DNS types are described in [client settings](client-features.md).

The engine resolves names through RL/2, prefers IPv4 when present, evaluates the ordered policy against the name and selected IP, then blocks, opens a protected direct socket, or requests a proxied connection to that numeric destination. Applications using their own DoH or literal IPs do not provide trustworthy names to this classifier.

There are 64 active client flow permits, bounded packet/UDP channels and fixed per-flow buffers. Limits do not constitute a denial-of-service audit of the upstream IP stack. The local loopback HTTP/CONNECT listener shares the same policy and permits. Its PAC delegates all traffic to this listener without a DIRECT fallback.

Heartbeat failure closes existing flows and retains capture during retries. Automatic reconnect backs off with jitter to roughly 30 seconds and resets after a stable minute. Persistent OS protection is separate from transport reconnect. Raw ICMP is not forwarded by RL/2. UDP-over-TCP still suffers head-of-line blocking; QUIC/CONNECT-UDP and congestion benchmarks remain future work.
