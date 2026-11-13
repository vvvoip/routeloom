# RouteLoom roadmap / 开发路线

English and Simplified Chinese are included across Windows, macOS, Linux, Android and iOS/iPadOS. A complete signed, physically validated five-platform release remains the goal.

## Implemented alpha milestones / 已实现

- [x] Shared Rust policy engine, explanatory CLI and bounded PAC export.
- [x] TLS 1.3 device authentication, server egress policy and legacy RL/1 compatibility.
- [x] Native capture bindings for all five platforms.
- [x] Shared RL/2 TCP/UDP engine with synthetic DNS domain classification.
- [x] Local HTTP/CONNECT proxy and live PAC endpoint using the same policy.
- [x] Automatic reconnect while retaining capture.
- [x] Persistent desktop filters and native mobile protection controls.
- [x] Protected saved profiles and bilingual preferences.
- [x] Signed update metadata, verified downloads and Windows/Linux/Android installation paths.
- [x] Bilingual website, downloadable Windows/Linux/Android packages and Apple signing kit.
- [x] Deployment and live connection testing on the owner's Debian 12 server.

## Remaining validation and distribution / 待验证与分发

- [ ] Signed Apple binaries and an App Store/TestFlight or supported macOS distribution channel.
- [ ] Physical Android/iPhone/iPad and signed Mac testing.
- [ ] Reboot, sleep/resume, network switching, revocation and competing-VPN checks across all five platforms.
- [ ] Windows signed installer and least-privilege service separation.
- [ ] Independent security review, fuzzing, performance and battery benchmarks.
- [ ] Broader Linux distributions, resolver integrations and architectures.

## Later capabilities / 后续功能

- [ ] SOCKS5, standard QUIC/CONNECT-UDP transport and congestion benchmarking.
- [ ] Invitation enrollment, live credential rotation/revocation and automatic certificate renewal.
- [ ] Licensed geographic datasets and application rules where supported.
- [ ] Expanded DNS record support and local/private-name resolution.
- [ ] Guided server backup, upgrades and rollback.

Payments, public VPN subscriptions, mesh/site-to-site networking and custom cryptographic primitives remain outside the initial scope.
