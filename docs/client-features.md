# Client settings
[简体中文](client-features.zh-CN.md) · [Installation](install.md)

Version 0.2 adds the same English/Chinese settings to Windows, macOS, Linux, Android and iOS/iPadOS. Apple apps still require your Xcode signature. This is an experimental alpha; build checks do not replace device testing.

## Profiles and routing

Import a private profile from the clipboard. It is saved in the operating system's protected credential storage. Select, rename, edit or delete profiles under **Saved profiles**. A locked/unavailable keyring is reported explicitly; a session-only import is then possible. Linux needs an unlocked Secret Service keyring. Android and Apple also keep a protected copy for the background VPN provider.

Disconnect before changing profiles or rules. Rules run in order; the first match wins. End with exactly one final action:

```text
domain ads.example.com block
domain-suffix example.com direct
cidr 192.168.0.0/16 direct
port 25 block
network udp proxy:default
final proxy:default
```

Supported selectors: exact domain, domain suffix, IPv4/IPv6 CIDR, port and TCP/UDP. Supported actions: direct, block, proxy:default. Other proxy groups, geolocation datasets and application selectors are not implemented.

The shared engine converts captured TCP/UDP into flows. RouteLoom DNS gives names synthetic addresses, so different domains sharing one destination IP keep distinct rules. DNS names are resolved through the encrypted server connection, including names whose eventual connection is direct. Private names known only to your local resolver are therefore not supported yet; use IP rules for those internal resources. Literal-IP or app-managed encrypted-DNS traffic follows IP/default rules. Domain matching is ASCII/punycode.

The encrypted local DNS mapping cache persists across restarts. It is not a browsing-history log, but it does contain encrypted domain mappings. It has a 4,096-name limit per profile and a 65,534-allocation lifetime limit across profiles. Unknown, corrupt or exhausted mappings fail closed; IDs are never reassigned to a different name. Do not delete this cache while applications retain synthetic DNS answers.

Only A/AAAA DNS records are synthesized; other record types receive empty answers. Raw ICMP and non-TCP/UDP protocols are not forwarded by this engine. Existing more-specific OS routes can bypass capture; enable protection to block unintended direct egress.

## PAC and terminals

While connected, the default PAC URL is http://127.0.0.1:12789/proxy.pac. Copy it from the app into a compatible browser's automatic proxy setting. The same port accepts HTTP and CONNECT. Terminal examples:

```sh
export http_proxy=http://127.0.0.1:12789
export https_proxy=http://127.0.0.1:12789
```

The PAC sends requests to the local policy engine, which applies domain/IP/port/protocol rules. There is no automatic DIRECT fallback. Arbitrary imported PAC JavaScript and SOCKS5 are not supported. Remove the browser/terminal proxy setting when you no longer want it; disconnecting stops the local listener. Port 0 in profile options disables the listener.

## Reconnect and persistent protection

Automatic reconnect retries with jittered exponential delays from about 1 to 30 seconds. Capture stays installed while the server/network is unavailable, existing flows close, and apps must retry their connections. No implicit direct fallback occurs inside the engine.

Protection is opt-in. Enable the kill switch before connecting. **Disconnect does not remove persistent protection.** Use **Restore direct internet** when you deliberately want ordinary networking again.

- Windows uses owned persistent Windows Filtering Platform rules. They survive the helper exiting; only the RouteLoom helper, loopback, tunnel interface and necessary network configuration traffic are allowed. Protection begins when enabled and depends on the OS firewall services.
- Linux uses an owned nftables table plus a systemd boot unit. It leaves unrelated firewall tables intact. Allowances include marked RouteLoom sockets, the tunnel, loopback and network configuration traffic.
- Android requires you to open system VPN settings and enable **Always-on VPN** and **Block connections without VPN**. The app switch cannot grant these OS settings. Turn off system lockdown to restore direct internet.
- Apple uses Network Extension includeAllNetworks and on-demand reconnection. A protected disconnect pauses forwarding while keeping capture. OS networking exceptions apply; this is not a claim of zero traffic during boot or every system transition. Personally signed builds require physical-device validation.

Explicit direct rules are intentional exceptions while the engine is connected. Sleep/wake, switching networks, OS updates, competing VPNs and containers require further device tests.

Recovery without the GUI:

```powershell
# Administrator PowerShell, in the extracted Windows app folder:
.\routeloom-vpn.exe unlock
```
```sh
# Linux:
sudo /usr/lib/routeloom/routeloom-vpn unlock
```

Restore direct internet **before uninstalling**. Removing the app alone does not remove persistent desktop filters.

## Updates

When enabled, RouteLoom checks GitHub every six hours while open. It verifies the pinned publisher Ed25519 signature, release sequence and SHA-256 before staging a package. Older signed sequences, substituted asset names and modified packages are rejected.

Windows installs the verified portable package after the app exits, preserving its state directory and attempting rollback if replacement fails. Linux opens the verified DEB in your package installer. Android opens the system APK installer and may request permission to install updates. Installation requires your action/OS approval.

All five platforms check for updates. Personally signed macOS/iOS builds require the new signing kit and Xcode; unattended Apple binary updates require a configured signed distribution channel. Apple signing kits are source, not installable app updates.

The 0.1 preview Android APK used a different signing key. Keep your private profile, uninstall that preview once, then install 0.2 and reimport. Later releases use the permanent project key. Windows still has no publisher code-signing certificate.
