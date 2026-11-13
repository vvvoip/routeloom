"""Real TCP/UDP traffic through the namespace VPN (no external network needed)."""
import socket
import sys
import threading
import time

def tcp_server(family, address):
    listener = socket.socket(family, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind((address, 18080)); listener.listen()
    while True:
        conn, _ = listener.accept()
        with conn:
            while True:
                data = conn.recv(4096)
                if not data: break
                conn.sendall(data)

def udp_server(family, address):
    server = socket.socket(family, socket.SOCK_DGRAM)
    server.bind((address, 18081))
    while True:
        data, peer = server.recvfrom(2048); server.sendto(data, peer)


def http_server():
    from http.server import BaseHTTPRequestHandler,HTTPServer
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            body=self.client_address[0].encode()
            self.send_response(200);self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
        def log_message(self,*args):pass
    HTTPServer(('192.0.2.1',18082),Handler).serve_forever()

if sys.argv[1] == "serve":
    threading.Thread(target=http_server,daemon=True).start()
    for family, address in [(socket.AF_INET, "10.77.0.1"), (socket.AF_INET6, "fd77::1")]:
        threading.Thread(target=tcp_server, args=(family,address), daemon=True).start()
        threading.Thread(target=udp_server, args=(family,address), daemon=True).start()
    while True: time.sleep(60)
elif sys.argv[1] == "test":
    for family, address in [(socket.AF_INET, "10.77.0.1"), (socket.AF_INET6, "fd77::1")]:
        with socket.socket(family, socket.SOCK_STREAM) as client:
            client.settimeout(5); client.connect((address,18080))
            payload = bytes(range(256))*16
            client.sendall(payload)
            received = b""
            while len(received)<len(payload): received += client.recv(4096)
            assert received == payload
        with socket.socket(family, socket.SOCK_DGRAM) as client:
            client.settimeout(5)
            payload = bytes(range(256))*2
            client.sendto(payload,(address,18081))
            assert client.recv(2048) == payload
    print("IPv4/IPv6 TCP and UDP traffic passed.")

# Domain identity must survive synthetic DNS and shared destination IPs.
def dns_address(host):
    import struct
    question = b''.join(bytes([len(part)])+part.encode() for part in host.split('.'))+b'\0'+struct.pack('!HH',1,1)
    query = struct.pack('!HHHHHH',1234,0x100,1,0,0,0)+question
    with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as client:
        client.settimeout(5);client.sendto(query,('10.77.0.1',53));reply=client.recv(4096)
    assert reply[:2] == query[:2] and int.from_bytes(reply[6:8],'big') == 1
    return socket.inet_ntoa(reply[-4:])
def http(address,host):
    with socket.create_connection((address,18082),timeout=5) as client:
        client.sendall(('GET / HTTP/1.1\r\nHost: '+host+'\r\nConnection: close\r\n\r\n').encode())
        data=b''
        while True:
            chunk=client.recv(4096)
            if not chunk:break
            data+=chunk
        return data
if sys.argv[1] == 'domain':
    direct=dns_address('direct.routeloom.test')
    proxy=dns_address('proxy.routeloom.test')
    blocked=dns_address('blocked.routeloom.test')
    assert len({direct,proxy,blocked})==3
    assert b'192.0.2.2' in http(direct,'direct.routeloom.test')
    assert b'192.0.2.1' in http(proxy,'proxy.routeloom.test')
    try:
        result=http(blocked,'blocked.routeloom.test')
        assert b'200 OK' not in result
    except (ConnectionError,TimeoutError,OSError):pass
    print('Domain DIRECT/PROXY/BLOCK decisions passed with a shared destination IP.')
