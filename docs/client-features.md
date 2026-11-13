# Client settings
[简体中文](client-features.zh-CN.md) · [Installation](install.md)

Version 0.4 provides the live traffic dashboard, Auto/Proxy/Direct modes, connection testing and a China/LAN routing preset on Windows, macOS, Linux, Android and iOS, in English and Simplified Chinese. Apple apps still require your Xcode signature. This is an experimental alpha; build checks do not replace device testing. See [upgrading an existing Apple installation](upgrade-apple.md).

## Profiles and routing

To use the bundled strategy, disconnect, select your saved profile in **Settings**, and choose **China and LAN direct**. This selects Auto and replaces that profile's routing policy; credentials, reconnect and protection settings remain intact. Custom rules require confirmation before replacement. If protection keeps the VPN blocked, use **Restore direct internet** first, then apply the preset and reconnect.

The preset sends local IPv4/IPv6 networks, `.cn`, Chinese IDN suffixes and a curated list of mainland services directly. Other destinations use your server. Selected overseas-service exceptions come before broader direct rules. This is not a complete China GeoIP/GeoSite database; literal IPs and app-managed encrypted DNS follow IP/default rules. Auto DNS resolution still uses the server, including names whose connections go direct. Private local names need a resolver that knows them. [Sources and license](routing-preset-sources.md).

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

The shared engine converts captured TCP/UDP into flows. RouteLoom DNS gives names synthetic addresses, so different domains sharing one destination IP keep distinct rules. In Auto and Proxy modes, DNS names are resolved through the encrypted server connection, including Auto rules whose eventual connection is direct. In Direct mode, names use the profile's configured DNS servers over the underlying network, without the RouteLoom server. Private names require a resolver that knows them; Auto/Proxy cannot use a local-only resolver. Literal-IP or app-managed encrypted-DNS traffic follows IP/default rules. Domain matching is ASCII/punycode.

The encrypted local DNS mapping cache persists across restarts. It is not a browsing-history log, but it does contain encrypted domain mappings. It has a 4,096-name limit per profile and a 65,534-allocation lifetime limit across profiles. Unknown, corrupt or exhausted mappings fail closed; IDs are never reassigned to a different name. Do not delete this cache while applications retain synthetic DNS answers.

Only A/AAAA DNS records are synthesized; other record types receive empty answers. Raw ICMP and non-TCP/UDP protocols are not forwarded by this engine. Existing more-specific OS routes can bypass capture; enable protection to block unintended direct egress.

## Modes, live connections and testing

On **Overview**, choose a mode before connecting. The choice is saved with the selected profile. Disconnect before changing it.

| Mode | Behavior |
| --- | --- |
| Auto | Apply the profile's rules in order, including block rules. This means rule-based routing, not automatic server selection or direct fallback. |
| Proxy | Send all captured TCP/UDP through the configured server, overriding direct and block rules. |
| Direct | Send all captured TCP/UDP directly and resolve names via the configured DNS servers on the underlying network, overriding proxy and block rules. The proxy server need not be available. |

The **Traffic** tab shows captured TCP/UDP connections, destination domain or IP and port, actual route, matched rule, state, duration, and upload/download counters. Search by host, address, source or rule; filter TCP/UDP, route, or active connections. Pause freezes the display. **Clear closed** removes completed rows while keeping active connections and session totals.

The latest 256 connection records remain in RAM only and disappear when the engine stops. This is separate from the encrypted DNS mapping cache described above. No packet contents, full URLs or credentials are recorded. Names are available for RouteLoom DNS or local-proxy requests; other traffic may show only an IP. Process names are not available. Counters represent forwarded TCP/UDP payload bytes, excluding DNS capture and tunnel overhead. Traffic outside RouteLoom's capture is not visible.

**Test connection** makes an explicit HTTPS request to api.ipify.org using the current engine and policy. It displays the selected mode, the actual route of that request, observed public IP, elapsed time and whether that IP matches the configured server address. The third-party service receives the request's exit IP. An IP match alone does not determine the route: the recorded engine decision does. This tests one request, not every application or a complete DNS/IPv6 leak audit.

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
