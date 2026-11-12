# RouteLoom roadmap / 开发路线

The first usable public release must include all five platforms together, in English and Simplified Chinese. CI build success is not a VPN release gate by itself.

## M0 — Foundation / 基础开发

- [x] Rust workspace, strict ordered policy engine, explanatory CLI, bounded PAC export.
- [x] Bilingual Flutter interface and native adapter contracts.
- [x] Bilingual static website and Pages workflow.
- [x] Native build matrix defined for Windows, macOS, Linux, Android, iOS.
- [ ] Native capture feasibility prototype on all five platforms.
- [ ] Apple entitlement and distribution feasibility confirmed.

## M1 — Working connection / 完整连接

- [ ] TLS 1.3 authenticated TCP tunnel and interop tests.
- [ ] Local HTTP CONNECT / SOCKS5 ingress.
- [ ] Enrollment invitations, per-device secrets, revocation and rotation.
- [ ] Native adapters, core bridge, route/DNS rollback.
- [ ] Server destination authorization, metadata/private-address protection.

## M2 — Coordinated beta / 统一测试版

- [ ] UDP / CONNECT-UDP, transport fallback and congestion benchmarks.
- [ ] DNS integration, IPv6, rule editing, supported PAC application.
- [ ] Versioned geodata, licensed datasets, app rules where supported.
- [ ] Guided server deployment, backup, updates and rollback.
- [ ] English/Chinese strings, documentation, and error catalog completed.

## M3 — Public release / 公开发布

- [ ] All five device platforms pass capture, routing, reconnect, sleep/resume, DNS/IPv6 leak, revocation and recovery tests.
- [ ] Physical iPhone/Android battery and background lifecycle tests.
- [ ] Security review, fuzzing, dependency inventory, signed release artifacts.
- [ ] Native installers, Apple signing and distribution, safe updater.
- [ ] Deploy and test on the owner's supplied server.

Payments, public VPN subscriptions, mesh/site-to-site networking and custom transport cryptography are outside the initial scope.
