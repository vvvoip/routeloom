# Stateful client milestone

Implementation work in progress. Do not publish a binary claiming these guarantees until its platform checks pass.

- Add a shared Rust flow engine for Windows/Linux and native mobile/Apple libraries, using a bounded smoltcp network stack. Virtual DNS assigns session-stable addresses to names; rules are evaluated per TCP/UDP flow. Unknown/stale virtual addresses are blocked, never reused for another name.
- Keep RL/1 for existing clients. Add TLS 1.3 ALPN routeloom/2 for individually authenticated TCP/UDP flows, DNS resolution and a heartbeat control connection. Retain server egress ACLs and per-device/global limits.
- Serve a loopback HTTP CONNECT/HTTP proxy and generated PAC from the same policy. No arbitrary PAC JavaScript is evaluated. Domain rules require the app to use RouteLoom DNS or the local proxy; opaque DoH/literal-IP traffic follows IP/default rules.
- Reconnect with bounded exponential backoff and retain capture while offline. No implicit direct fallback.
- Persistent blocking: Windows WFP persistent filters, Linux owned nftables table and boot unit; Android system always-on/lockdown settings; Apple includeAllNetworks/on-demand with the OS's documented exceptions. Explicit direct rules are intentional exceptions when connected.
- Saved profiles use platform secure storage. Android/Apple providers retain encrypted/Keychain credentials for OS restart. Never save plaintext tokens or log destinations.
- Automatic update checking and verified staged downloads; installation remains subject to OS permissions, signing and store rules. Apple personal development builds require the user's Xcode signing flow.
- UI, instructions, errors and settings are supplied in English and Simplified Chinese.
