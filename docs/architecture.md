# Architecture

Status: experimental 0.2 alpha. [Installation](install.md) · [Client settings and limits](client-features.md) · [RL/2](rl2.md) · [Legacy RL/1](rl1.md).

- Rust policy core validates ordered domain/CIDR/port/protocol rules. It is independent of network access.
- The shared Rust engine classifies TCP/UDP flows using synthetic DNS mappings. Direct sockets bypass capture; proxy sockets use authenticated TLS 1.3 RL/2; blocked flows never dial their destination.
- Windows/Linux use a Rust Wintun/TUN helper. Profiles travel over stdin, not process arguments. Owned routes/DNS are removed on ordinary disconnect. Persistent WFP/nftables protection has a separate explicit unlock operation.
- Android uses JNI, VpnService.protect, foreground lifecycle and AndroidKeyStore credentials. OS always-on/lockdown must be enabled by the user.
- macOS/iOS use the same Rust library in a Network Extension. Keychain credentials support provider restart, includeAllNetworks supports protection, and on-demand configuration supports recovery. Apple signing and real-device tests remain external gates.
- The server accepts both RL/2 flows and RL/1 packets. It validates device credentials, bounds connections, rejects unauthorized private/metadata egress and preserves legacy lease checks. RL/1 uses Linux TUN/NAT; RL/2 uses connected server sockets.
- Flutter provides English/Chinese profiles, rule editing, connection status, protection controls and verified update handling. Profile storage uses the operating system's protected vault.
- The website is static bilingual GitHub Pages content, separate from the Debian tunnel host.

Application → OS capture → shared TCP/UDP flow engine → ordered rule → direct / block / RL/2 TLS server → destination.

PAC/HTTP CONNECT enters the same engine over loopback. Arbitrary PAC JavaScript, SOCKS5, geographic datasets, application/process rules, automatic credential enrollment and QUIC transport are not implemented. A country rule alone would not create a geographic exit node.

Capture persists during automatic reconnect. A kill switch must be explicitly enabled and follows platform-specific rules. There is no universal zero-leak claim across boot, every OS service, sleep/resume, other VPNs, containers or virtual machines. See the recovery instructions before enabling protection.

Updates verify an Ed25519-signed manifest, monotonic sequence and asset hashes. Windows replaces a staged portable package with rollback; Linux/Android delegate installation to the OS. Personally signed Apple builds check for updates but still require Xcode installation.
