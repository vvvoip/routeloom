# Development

Use Rust 1.100.0 (rust-toolchain.toml), Flutter 3.50.0 (.fvmrc), and Node.js 24. The Rust foundation has no third-party dependencies. Install native compiler requirements for your OS. Flutter Windows builds need Visual Studio C++ tools; Apple builds need macOS/Xcode. Android needs an SDK/JDK.

## Rust

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo run -p routeloom-cli -- check examples/default.rules
cargo run -p routeloom-cli -- explain examples/default.rules example.org
cargo run -p routeloom-cli -- --lang zh-CN explain examples/default.rules blocked.example
cargo run -p routeloom-cli -- pac examples/browser.rules 127.0.0.1:7890
cargo run -p routeloom-server -- serve-health
```

The last command exposes a loopback-only health scaffold, not a proxy. It returns tunnel_ready=false. No cloud deployment yet. CLI technical diagnostics are currently English even when result labels are Chinese.

The v0 rules syntax is intentionally small: `domain NAME ACTION`, `domain-suffix NAME ACTION`, `cidr RANGE ACTION`, `port NUMBER ACTION`, `network tcp|udp ACTION`, then `final ACTION`. Actions are `direct`, `block`, `proxy:NAME`. Lines starting with # are comments. ASCII/Punycode hostnames only. Resolved IP metadata is caller supplied. A named proxy is a policy result, not a configured working server.

PAC export rejects block, IP/port/protocol rules and non-default proxy groups; it never drops unsupported rules or appends a DIRECT fallback. It does not start a local proxy listener.

## Client

With Flutter on PATH:

```sh
node scripts/bootstrap-client.mjs
cd apps/client
flutter pub get
flutter gen-l10n
flutter test
flutter run
```

The bootstrap script generates official runners for all five platforms in .local/client-runners, then copies missing runners into apps/client. Existing native edits are preserved. The generated runners are the starting point for native integration, not completed VPN adapters. The app never requests VPN permission in this build.

Translation source: lib/l10n/app_en.arb and app_zh.arb. Generated localization files are ignored. Language selection currently lasts for the application session; persistence is future work.

## Website

```sh
node website/build.mjs
node --test scripts/site.test.mjs
```

Output: website/dist, base path /routeloom. No npm dependencies. GitHub Pages must use **Settings → Pages → Source → GitHub Actions**. The Pages workflow deploys on main changes to the site. Deployment does not need the future proxy server.

## Verification limits

Widget tests that override TargetPlatform verify interface behavior, not OS networking. Native CI build jobs verify shell compilation, not app-store readiness. The optional scripts/run-wasi.mjs runs pure Rust tests on machines without a native linker; it is not a replacement for native networking tests.
