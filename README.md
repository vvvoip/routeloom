# RouteLoom

[English](README.md) · [简体中文](README.zh-CN.md) · [Website](https://vvvoip.github.io/routeloom/)

An open-source, self-hosted proxy platform in early development. One shared routing core, five client platforms, and a server you control.

**Status: VPN developer milestone / pre-alpha. Native capture and TLS 1.3 transport are implemented; signed real-device validation and production packaging are still required.** The first public usable release is gated on Windows, macOS, Linux, Android, and iOS/iPadOS together. English and Simplified Chinese are launch requirements.

## Development

- Rust workspace: routing policy/PAC CLI, authenticated IPv4/IPv6 packet transport, Linux tunnel server, and Windows/Linux capture helper.
- Flutter client: English/Chinese profile import and connection controls; Android VpnService and shared macOS/iOS Network Extension targets.
- Static bilingual website: built with Node.js, published through GitHub Actions to Pages.

Start with the [VPN development guide](docs/vpn-development.md) and [RL/1 protocol](docs/rl1.md). Domain rules are not yet wired into the VPN packet path.

See [architecture](docs/architecture.md), [roadmap](docs/roadmap.md), [platform matrix](docs/platforms.md), and [development guide](docs/development.md). CI checks source and build foundations; it does not establish VPN readiness.

## Principles

Explicit routing and DNS policies. Established encryption. Device-specific credentials. No browsing-history logging by default. Honest platform limits. No central account required for the planned self-hosted product.

## License

Apache-2.0 for original code; third-party components retain their own licenses. Review [security reporting](SECURITY.md) before disclosing a vulnerability.
