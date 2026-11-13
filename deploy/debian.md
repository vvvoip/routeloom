# Debian 12 cloud deployment

The VPN server was built and tested on Debian 12 amd64 (glibc 2.36). Use a Debian-native build: binaries built on newer glibc distributions may not run on Debian 12.

## Preparation

Use a dedicated test host with /dev/net/tun, iproute2, nftables, OpenSSL 3, Python 3 and a C compiler. Check existing listeners, interfaces, firewall rules and forwarding settings before changing anything. Keep provider console access available.

Build the committed source with Rust 1.100.0 and Cargo.lock. Use a separate unprivileged build account and verify the official rustup installer checksum. Commands inside the source directory:

```sh
cargo test --workspace --locked
cargo build --release -p routeloom-vpn --locked
```

Install the binary as root-owned /opt/routeloom/routeloom-vpn. Create a system account named routeloom with no login shell. Place server.json, the TLS certificate and private key in /etc/routeloom, owned by root:routeloom, directory mode 0750 and file mode 0640. Do not publish profiles or keys.

## Service and networking

The [service template](systemd/routeloom.service) runs as routeloom with CAP_NET_ADMIN for TUN configuration, a read-only filesystem view, a device allow-list, bounded tasks/memory, and SIGINT cleanup. It listens on the port configured in server.json; the documented default is TCP 9443.

Review [network.nft.example](network.nft.example), substitute the uplink interface and server public IPv4, and install it as /etc/routeloom/network.nft. It blocks tunnel access to host services, private/metadata destinations and the host's own public address. Its NAT rule applies only to the RouteLoom lease subnet. It does not flush other nftables tables.

The example supports IPv4 Internet egress. It rejects forwarded IPv6 because no upstream IPv6 is assumed. On a server with no public IPv6 route, keep IPv6 captured and report it as unavailable; do not advertise working IPv6 egress. Use IPv4 DNS servers in client profiles.

Enabling global IPv4 forwarding changes host routing behavior. If it was previously disabled and the host has no other routing purpose, add a final drop rule for other forwarded traffic to preserve that behavior. If another service already routes packets, integrate with its firewall deliberately.

Copy both systemd units, then validate and start:

```sh
nft --check --file /etc/routeloom/network.nft
systemd-analyze verify /etc/systemd/system/routeloom.service /etc/systemd/system/routeloom-network.service
systemctl daemon-reload
systemctl enable --now routeloom-network.service
systemctl enable --now routeloom.service
```

Allow the tunnel TCP port in the cloud security group. Restrict source IPs when practical. Host rules cannot override a provider firewall. Keep SSH and unrelated services intact.

## Verification

`node scripts/probe-server.mjs PRIVATE_PROFILE.json` runs on an external machine and checks TLS 1.3, ALPN, device authentication, tunneled UDP DNS and keepalive. It generates packets inside TLS without installing a local VPN interface or changing the workstation's routes/DNS.

Use an isolated Linux network namespace for full client capture tests. Verify HTTPS, a large response against its direct-download hash, UDP DNS, blocked metadata, failure behavior and TUN cleanup. A namespace-specific resolv.conf can supply DNS while resolvectl is stubbed; this does not validate desktop resolver integration.

Restart the service and repeat the external probe. Remove temporary test identities from server.json, restart, and verify the old profile is rejected. Keep one distinct profile per device.

## Operations

- Status: `systemctl status routeloom`; logs: `journalctl -u routeloom`.
- Restart tunnel: `systemctl restart routeloom`.
- Restart networking: restart routeloom-network, then start routeloom again (the dependency stops it when network rules are stopped).
- Revoke a device: remove its server.json entry and restart. Active sessions close.
- Stop and disable: `systemctl disable --now routeloom.service routeloom-network.service`. This deletes only the owned tables.
- Restore the previous forwarding value separately, after confirming no other service now depends on it.
- Certificate renewal is manual. Record the expiry and distribute new profiles when rotating a pinned certificate.

These templates were tested with systemd on Debian 12. They are a developer deployment, not a general-purpose installer or a substitute for signed-device and DNS-leak validation.

## 简体中文

Debian 12 amd64 已完成原生编译和隧道验证。使用专用构建用户编译，并通过独立的 routeloom 服务用户运行。仅授予创建 TUN 所需的 CAP_NET_ADMIN，配置和私钥仅供 root 与服务组读取。

云安全组必须开放所配置的 TCP 端口（默认 9443）。防火墙规则只管理 RouteLoom 自己的表，保留 SSH 和其他服务。服务器没有公网 IPv6 时，IPv6 流量保持捕获但无法访问互联网；不要将其描述为支持 IPv6 出口。

从外部机器运行 probe-server.mjs 验证 TLS、认证、DNS 和保活。使用隔离网络命名空间验证真实捕获、HTTPS、较大文件、元数据阻断和清理。测试后撤销临时设备。生产设备使用独立配置；私钥和设备配置不得提交到仓库。证书需手动更新，记录到期时间并重新分发包含新证书的配置。
