# RouteLoom

[English](README.md) · [简体中文](README.zh-CN.md) · [Website](https://vvvoip.github.io/routeloom/)

An open-source, self-hosted proxy platform in early development. One shared routing core, five client platforms, and a server you control.

**Status: experimental VPN alpha.** [Download client previews](https://github.com/vvvoip/routeloom/releases) and follow the [installation guide](docs/install.md). Windows, Ubuntu and Android packages are provided; Apple platforms currently require the source signing kit and your developer account. English and Simplified Chinese are included. A complete signed five-platform release remains the goal.

## Development

- Rust workspace: routing policy/PAC CLI, authenticated IPv4/IPv6 packet transport, Linux tunnel server, and Windows/Linux capture helper.
- Flutter client: English/Chinese profile import and connection controls; Android VpnService and shared macOS/iOS Network Extension targets.
- Static bilingual website: built with Node.js, published through GitHub Actions to Pages.

Start with the [VPN development guide](docs/vpn-development.md) and [RL/1 protocol](docs/rl1.md). The shared [RL/2 flow engine](docs/rl2.md) applies domain/IP rules to TCP/UDP, with PAC, reconnect, protected profiles and signed update checks. Read [client settings and recovery](docs/client-features.md).

See [architecture](docs/architecture.md), [roadmap](docs/roadmap.md), [platform matrix](docs/platforms.md), and [development guide](docs/development.md). CI checks source and build foundations; it does not establish VPN readiness.

## Principles

Explicit routing and DNS policies. Established encryption. Device-specific credentials. No persistent connection logs; recent connection metadata stays in memory. The encrypted DNS mapping cache persists locally. Honest platform limits. No central account required for the planned self-hosted product.

## License

Apache-2.0 for original code; third-party components retain their own licenses. Review [security reporting](SECURITY.md) before disclosing a vulnerability.
