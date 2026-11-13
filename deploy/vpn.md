# Dedicated Linux test server

Use a dedicated Debian 12 or Ubuntu 24.04 test VM with a public IP, /dev/net/tun, root/sudo and inbound TCP 9443. One vCPU and 1–2 GB RAM is sufficient for initial functional testing; throughput has not been benchmarked. Keep the existing GitHub Pages website free and independent.

For the tested Debian service setup, see [Debian deployment](debian.md).

Create profiles as described in [VPN development](../docs/vpn-development.md). Restrict SSH to your administrative IP and preserve provider console access. The commands below change networking: run them only on the dedicated test server, not an existing production gateway.

For initial **IPv4 Internet egress**, enable forwarding and install narrowly scoped nftables rules. Replace eth0 with the real external interface after inspecting `ip route`. Integrate these rules with your existing firewall; do not flush its ruleset.

```sh
sudo sysctl -w net.ipv4.ip_forward=1
sudo nft add table ip routeloom_nat
sudo nft 'add chain ip routeloom_nat postrouting { type nat hook postrouting priority srcnat; policy accept; }'
sudo nft add rule ip routeloom_nat postrouting ip saddr 10.77.0.0/24 oifname eth0 masquerade
```

The host's forwarding firewall must permit rl-server → external egress and established return traffic, and reject access to management/metadata/private destinations unless deliberately allowed. The application's destination checks are an additional layer, not a replacement for firewall review.

For IPv6, use a routed prefix supplied by the provider or configure an equivalent NAT66 rule for fd77::/64 and enable `net.ipv6.conf.all.forwarding=1`. Without working IPv6 egress, full-tunnel profiles still capture IPv6 but those connections fail rather than becoming a deliberate direct IPv6 fallback. Prefer provider-routed IPv6 in production.

These commands are temporary developer setup. Do not enable forwarding or NAT on the user's machine automatically. A reviewed installer, least-privilege daemon, persistent firewall configuration, certificate renewal and deployment rollback remain release gates.

Run the server in a terminal for initial testing. To revoke a device, remove it from server.json and restart: existing sessions are closed. Never expose the old development health scaffold publicly; it is not a control API. The server has no remote admin endpoint.

## Bring-up checklist

1. Generate a unique device profile; transfer it privately.
2. Start the server and open only the selected tunnel TCP port.
3. Test one OS at a time with browser and terminal traffic, then UDP/DNS and IPv6.
4. Confirm source-address enforcement and blocked metadata/private egress.
5. Stop the server and verify the documented disconnect behavior.
6. Test device sleep, network changes and clean route/DNS restoration.
7. Record OS versions and results. Do not call the build production-ready based on compilation.

服务器用于下一步真实网络和性能验证。请提供独立测试主机的系统版本、公网 IP、SSH 用户、端口和已有密钥的本地路径；不要把密码或私钥粘贴到公开仓库。网站可继续使用免费的 GitHub Pages。
