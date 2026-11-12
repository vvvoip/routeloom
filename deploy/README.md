# Deployment status / 部署状态

There is no production installer yet. The server binary currently supports only `serve-health [127.0.0.1:9080]`, refuses non-loopback listeners, and always reports `tunnel_ready: false`.

Before cloud deployment: implement authentication and TLS, enrollment/revocation, destination authorization (including cloud metadata/private IP protection), resource limits, redacted logs, systemd lifecycle, signed version selection, backup and rollback. Then test on the owner-provided server using a restricted deployment account.

GitHub Pages hosts the official static site independently; website publication does not deploy or enable proxy traffic. No server access is needed at this stage.

当前尚无生产安装器。服务端仅提供本地健康检查，不能代理流量。完成认证、加密、权限控制、资源限制与回滚机制后，再使用用户提供的服务器测试部署。
