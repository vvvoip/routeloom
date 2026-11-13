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

if sys.argv[1] == "serve":
    for family, address in [(socket.AF_INET, "10.77.0.1"), (socket.AF_INET6, "fd77::1")]:
        threading.Thread(target=tcp_server, args=(family,address), daemon=True).start()
        threading.Thread(target=udp_server, args=(family,address), daemon=True).start()
    while True: time.sleep(60)
else:
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
