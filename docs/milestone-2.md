# Stateful client milestone validation

Version 0.2 implements shared RL/2 domain/IP routing, PAC/HTTP CONNECT, reconnect, persistent OS protection controls, protected saved profiles and signed update handling. English and Simplified Chinese share the same settings. See [features and limitations](client-features.md).

## Verified during development

- Local Windows Rust workspace: 30 tests pass, including TLS rejection, policy precedence, encrypted DNS-journal persistence/profile separation, archive-path confinement and protecting a Windows socket before binding.
- Flutter: 15 tests pass, including secure profile persistence, failed vault writes, signed update tampering/rollback/path rejection and five-platform UI rendering.
- Bilingual website/localization/release-metadata checks pass.
- Linux CI namespaces: IPv4/IPv6 TCP/UDP capture, direct/proxy/blocked domains sharing one IP, PAC, automatic recovery after server suspension, protection after helper disconnect, preservation of unrelated firewall tables, unlock recovery and invalid credentials/private destinations.
- Windows CI: packaged application launch, persistent WFP block after the installing process exits, explicit unlock recovery, successful package replacement, hash rejection, unsafe archive rejection, failed replacement rollback and preservation of private state.
- Live Windows Wintun against the authorized Debian 12 cloud server: new RL/2 HTTPS/public exit address, DNS, PAC and HTTP CONNECT pass; normal disconnect removes the owned adapter and routes. Persistent filters were tested only on the disposable CI runner, not on the owner’s workstation/server.
- Upgraded cloud service: legacy RL/1 TLS/DNS/heartbeat compatibility and RL/2 rejection of wrong credentials/unauthorized private destinations pass. The unrelated existing proxy service remains active.

## Remaining gates

The latest commit must pass the five-platform build workflow before publication. Earlier unsigned Apple builds passed; updated Apple builds still require CI and then the owner’s Xcode signing and physical-device checks. Android physical-device validation is also pending. Reboot, sleep/wake, network changes, battery behavior, competing VPNs and independent security review are not established by these tests.

Personally signed Apple clients check for releases but require Xcode to install updates. Android OS lockdown is a user setting. See [platform matrix](platforms.md) for distribution limits.
