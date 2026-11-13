"""Run in the disposable client namespace: assert real egress and private IPC telemetry."""
import http.client
import json
import queue
import socket
import struct
import subprocess
import sys
import threading
import time

binary, profile_file = sys.argv[1:]
base = json.load(open(profile_file))
for mode in ["auto", "proxy", "direct"]:
    profile = json.loads(json.dumps(base))
    profile["options"]["mode"] = mode
    profile["options"]["pac_port"] = 12791
    profile["dns"] = ["192.0.2.53"]
    profile["routes"].append("192.0.2.53/32")
    if mode == "direct":
        profile["endpoint"] = "192.0.2.1:9"  # No RouteLoom server listens here.
    process = subprocess.Popen([binary, "connect-stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
    messages = queue.Queue()
    def read_output():
        for line in process.stdout:
            try: messages.put(json.loads(line))
            except ValueError: pass
        messages.put({"eof": True})
    threading.Thread(target=read_output, daemon=True).start()
    def send(text):
        process.stdin.write(text + "\n"); process.stdin.flush()
    def wait_for(key, expected=None):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            item = messages.get(timeout=max(.1, deadline-time.monotonic()))
            if item.get("eof"): raise AssertionError("Helper exited before " + key)
            if key in item and (expected is None or item[key] == expected): return item[key]
        raise AssertionError("Timed out waiting for " + key)
    def request(host):
        client = http.client.HTTPConnection("127.0.0.1", 12791, timeout=10)
        try:
            client.request("GET", "http://" + host + ":18082/")
            reply = client.getresponse()
            return reply.status, reply.read().decode()
        finally: client.close()
    try:
        send(json.dumps(profile)); wait_for("state", "connected")
        expected = "192.0.2.2" if mode in ("auto", "direct") else "192.0.2.1"
        assert request("direct.routeloom.test") == (200, expected)
        blocked_status, _ = request("blocked.routeloom.test")
        assert blocked_status == (502 if mode == "auto" else 200)
        # A captured UDP echo must be reported with both byte counters.
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as client:
            client.settimeout(5)
            question=b"\x05proxy\x09routeloom\x04test\x00"+struct.pack("!HH",1,1)
            query=struct.pack("!HHHHHH",5371,0x100,1,0,0,0)+question
            client.sendto(query,("192.0.2.53",53)); reply=client.recv(4096)
            target=socket.inet_ntoa(reply[-4:])
            payload=b"monitor-udp-payload"
            client.sendto(payload,(target,18081))
            assert client.recv(2048)==payload
        time.sleep(.2)
        send("snapshot"); snapshot=wait_for("telemetry")
        assert snapshot["mode"] == mode
        assert snapshot["upload"] > 0 and snapshot["download"] > 0
        rows=snapshot["connections"]
        direct=next(r for r in rows if r["host"]=="direct.routeloom.test")
        assert direct["route"] == ("proxy" if mode=="proxy" else "direct")
        assert (direct["rule_line"] is not None) == (mode=="auto")
        blocked=next(r for r in rows if r["host"]=="blocked.routeloom.test")
        assert blocked["status"] == ("blocked" if mode=="auto" else "closed")
        udp=next(r for r in rows if r["protocol"]=="udp" and r["host"]=="proxy.routeloom.test")
        assert udp["upload"] == len(payload) and udp["download"] == len(payload)
        assert udp["route"] == ("direct" if mode=="direct" else "proxy")
        encoded=json.dumps(snapshot)
        assert profile["token"] not in encoded and "ca_pem" not in encoded
        send("clear");send("snapshot");cleared=wait_for("telemetry")
        assert all(r["status"] in ("active","connecting") for r in cleared["connections"])
        assert cleared["upload"] == snapshot["upload"]
        print(mode + ": egress, TCP/UDP counters, rule evidence, history clearing and credential exclusion passed.")
        send("disconnect");process.stdin.close()
        assert process.wait(timeout=10)==0
    finally:
        if process.poll() is None: process.kill();process.wait(timeout=5)
print("Mode and traffic monitor integration passed; Direct used an unavailable proxy endpoint.")
