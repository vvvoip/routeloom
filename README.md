# RouteLoom

[English](README.md) · [简体中文](README.zh-CN.md) · [Website](https://vvvoip.github.io/routeloom/)

An open-source, self-hosted proxy platform in early development. One shared routing core, five client platforms, and a server you control.

**Status: foundation / pre-alpha. There is no working VPN tunnel or installable production release yet.** The first public usable release is gated on Windows, macOS, Linux, Android, and iOS/iPadOS together. English and Simplified Chinese are launch requirements.

## Development

- Rust workspace: deterministic routing policy, CLI, and a loopback-only server health scaffold.
- Flutter client: bilingual interface and explicit platform capability reporting; connection stays unavailable until native adapters and transport are implemented.
- Static bilingual website: built with Node.js, published through GitHub Actions to Pages.

See [architecture](docs/architecture.md), [roadmap](docs/roadmap.md), [platform matrix](docs/platforms.md), and [development guide](docs/development.md). CI checks source and build foundations; it does not establish VPN readiness.

## Principles

Explicit routing and DNS policies. Established encryption. Device-specific credentials. No browsing-history logging by default. Honest platform limits. No central account required for the planned self-hosted product.

## License

Apache-2.0 for original code; third-party components retain their own licenses. Review [security reporting](SECURITY.md) before disclosing a vulnerability.
