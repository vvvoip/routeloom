#!/usr/bin/env bash
# Root-only, isolated namespaces. DNS command is stubbed: this tests IP capture, not the OS resolver.
set -euo pipefail
test "$(id -u)" = 0
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/routeloom-vpn"
WORK="$(mktemp -d)"
CLIENT="rlc-$$"
SERVER="rls-$$"
cleanup() {
  for ns in "$CLIENT" "$SERVER"; do
    ip netns pids "$ns" 2>/dev/null | xargs -r kill || true
    ip netns del "$ns" 2>/dev/null || true
    rm -f "/etc/netns/$ns/hosts"; rmdir "/etc/netns/$ns" 2>/dev/null || true
  done
  rm -rf -- "$WORK"
}
trap cleanup EXIT
node "$ROOT/scripts/create-dev-profile.mjs" "$WORK/config" 192.0.2.1:9443 localhost
node - "$WORK/config" <<'JS'
const fs=require('fs'),path=require('path');const dir=process.argv[2];
const s=JSON.parse(fs.readFileSync(path.join(dir,'server.json')));s.allow_private=['10.77.0.1/32','fd77::1/128','192.0.2.1/32'];
fs.writeFileSync(path.join(dir,'server.json'),JSON.stringify(s));
const p=JSON.parse(fs.readFileSync(path.join(dir,'device.profile.json')));p.routes=['10.77.0.1/32','fd77::1/128'];p.dns=['10.77.0.1'];p.options={policy:'domain direct.routeloom.test direct\ndomain blocked.routeloom.test block\nfinal proxy:default',auto_reconnect:true,kill_switch:false,pac_port:12789};
fs.writeFileSync(path.join(dir,'device.profile.json'),JSON.stringify(p));
JS
mkdir "$WORK/bin"
printf '#!/bin/sh\nexit 0\n' > "$WORK/bin/resolvectl"
chmod 700 "$WORK/bin/resolvectl"
ip netns add "$CLIENT"
ip netns add "$SERVER"
mkdir -p "/etc/netns/$SERVER"
printf "127.0.0.1 localhost\n192.0.2.1 direct.routeloom.test proxy.routeloom.test blocked.routeloom.test\n" > "/etc/netns/$SERVER/hosts"
ip link add "vethc-$" type veth peer name "veths-$"
ip link set "vethc-$" netns "$CLIENT"
ip link set "veths-$" netns "$SERVER"
ip -n "$CLIENT" addr add 192.0.2.2/24 dev "vethc-$"
ip -n "$SERVER" addr add 192.0.2.1/24 dev "veths-$"
ip -n "$SERVER" addr add 192.0.2.53/24 dev "veths-$"
ip -n "$CLIENT" link set "vethc-$" up
ip -n "$SERVER" link set "veths-$" up
ip -n "$CLIENT" link set lo up
ip -n "$SERVER" link set lo up
ip netns exec "$SERVER" "$BIN" serve "$WORK/config/server.json" > "$WORK/server.log" 2>&1 &
SERVER_PID=$!
sleep 1
ip netns exec "$CLIENT" env PATH="$WORK/bin:$PATH" "$BIN" connect "$WORK/config/device.profile.json" > "$WORK/client.log" 2>&1 &
CLIENT_PID=$!
for attempt in $(seq 1 30); do
  if grep -q '"connected"' "$WORK/client.log"; then break; fi
  if ! kill -0 "$CLIENT_PID" 2>/dev/null; then cat "$WORK/client.log" "$WORK/server.log"; exit 1; fi
  sleep 0.5
done
grep -q '"connected"' "$WORK/client.log" || { cat "$WORK/client.log" "$WORK/server.log"; exit 1; }
ip netns exec "$CLIENT" node "$ROOT/scripts/test-server-rejection.mjs" "$WORK/config/device.profile.json" flow-token
ip netns exec "$CLIENT" node "$ROOT/scripts/test-server-rejection.mjs" "$WORK/config/device.profile.json" flow-private
ip netns exec "$SERVER" python3 "$ROOT/scripts/vpn-traffic-test.py" serve > "$WORK/traffic.log" 2>&1 &
sleep 0.5
ip netns exec "$CLIENT" python3 "$ROOT/scripts/vpn-traffic-test.py" test
ip netns exec "$CLIENT" python3 "$ROOT/scripts/vpn-traffic-test.py" domain
# Transport loss keeps the client interface alive, then reconnects automatically.
kill -STOP "$SERVER_PID"
sleep 32
test -n "$(ip -n "$CLIENT" -o link show type tun)"
grep -q '"reconnecting"' "$WORK/client.log"
kill -CONT "$SERVER_PID"
for attempt in $(seq 1 70); do
  if [ "$(grep -c '"connected"' "$WORK/client.log")" -ge 2 ]; then break; fi
  sleep 1
done
test "$(grep -c '"connected"' "$WORK/client.log")" -ge 2
ip netns exec "$CLIENT" python3 "$ROOT/scripts/vpn-traffic-test.py" test
# Apply only inside the isolated client namespace; preserve an unrelated table.
TUN_NAME=$(ip -n "$CLIENT" -o link show type tun | awk -F': ' 'NR==1 {print $2}')
"$BIN" guard-rules "$TUN_NAME" > "$WORK/guard.nft"
ip netns exec "$CLIENT" nft add table inet routeloom_test_sentinel
ip netns exec "$CLIENT" nft -f "$WORK/guard.nft"
ip netns exec "$CLIENT" python3 "$ROOT/scripts/vpn-traffic-test.py" domain
if ip netns exec "$CLIENT" timeout 3 curl --noproxy '*' -fsS http://192.0.2.1:18082; then echo "Guard leaked underlay traffic"; exit 1; fi
kill -INT "$CLIENT_PID"
wait "$CLIENT_PID"
test -z "$(ip -n "$CLIENT" -o link show type tun)"
# The filter remains after the client process and TUN interface have gone.
if ip netns exec "$CLIENT" timeout 3 curl --noproxy '*' -fsS http://192.0.2.1:18082; then echo "Guard disappeared on disconnect"; exit 1; fi
ip netns exec "$CLIENT" nft list table inet routeloom_test_sentinel >/dev/null
ip netns exec "$CLIENT" nft delete table inet routeloom_guard
ip netns exec "$CLIENT" curl --noproxy '*' -fsS http://192.0.2.1:18082 >/dev/null
ip netns exec "$CLIENT" env PATH="$WORK/bin:$PATH" python3 "$ROOT/scripts/test-monitor.py" "$BIN" "$WORK/config/device.profile.json"
ip netns exec "$CLIENT" node "$ROOT/scripts/test-server-rejection.mjs" "$WORK/config/device.profile.json" token
ip netns exec "$CLIENT" node "$ROOT/scripts/test-server-rejection.mjs" "$WORK/config/device.profile.json" spoof
kill -INT "$SERVER_PID"
wait "$SERVER_PID"
echo "IPv4/IPv6 capture, encrypted round trip, and graceful interface cleanup passed."
