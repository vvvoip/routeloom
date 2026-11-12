# Contributing

Start with a roadmap issue. Keep changes small and describe the observable behavior and validation. New routing behavior needs meaningful boundary/security tests. Update English and Simplified Chinese together for user-facing changes.

Run cargo test --workspace, cargo fmt --all -- --check, flutter test in apps/client, and the website build/link checks for affected components. Native build checks are separate from real-device VPN tests. Never commit signing keys, device tokens, server access details or personal browsing data.

Original contributions are licensed under Apache-2.0. Retain third-party notices. Do not import a copyleft networking core without reviewing the project distribution plan and licenses first.
