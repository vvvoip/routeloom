import NetworkExtension
import Network
import Security

final class PacketTunnelProvider: NEPacketTunnelProvider {
    private let queue = DispatchQueue(label: "org.routeloom.packet")
    private var connection: NWConnection?
    private var timer: DispatchSourceTimer?
    private var completion: ((Error?) -> Void)?
    private var stopped = false
    private var ready = false
    private var lastReceived = Date()
    private var started = Date()
    private var profile: TunnelProfile?
    private var pendingWrites: [UInt64: Date] = [:]
    private var writeID: UInt64 = 0

    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        queue.async {
            self.completion = completionHandler
            do {
                guard let raw = options?["profile"] as? String, let data = raw.data(using: .utf8) else { throw TunnelError.invalid }
                let p = try TunnelProfile.parse(data)
                let anchors = try p.anchors()
                self.profile = p
                let tls = NWProtocolTLS.Options()
                sec_protocol_options_set_min_tls_protocol_version(tls.securityProtocolOptions, .TLSv13)
                sec_protocol_options_set_max_tls_protocol_version(tls.securityProtocolOptions, .TLSv13)
                sec_protocol_options_set_tls_server_name(tls.securityProtocolOptions, p.server_name)
                sec_protocol_options_add_tls_application_protocol(tls.securityProtocolOptions, "routeloom/1")
                sec_protocol_options_set_verify_block(tls.securityProtocolOptions, { _, trust, done in
                    let secTrust = sec_trust_copy_ref(trust).takeRetainedValue()
                    let policy = SecPolicyCreateSSL(true, p.server_name as CFString)
                    guard SecTrustSetPolicies(secTrust, policy) == errSecSuccess,
                          SecTrustSetAnchorCertificates(secTrust, anchors as CFArray) == errSecSuccess,
                          SecTrustSetAnchorCertificatesOnly(secTrust, true) == errSecSuccess else { done(false); return }
                    done(SecTrustEvaluateWithError(secTrust, nil))
                }, self.queue)
                let parameters = NWParameters(tls: tls, tcp: NWProtocolTCP.Options())
                let conn = NWConnection(host: NWEndpoint.Host(p.host), port: NWEndpoint.Port(rawValue: p.port)!, using: parameters)
                self.connection = conn
                conn.stateUpdateHandler = { [weak self] state in
                    guard let self = self, !self.stopped else { return }
                    switch state {
                    case .ready: self.hello(p)
                    case .failed(_), .cancelled: self.fail(TunnelError.connection)
                    default: break
                    }
                }
                self.started = Date(); self.lastReceived = Date()
                let timer = DispatchSource.makeTimerSource(queue: self.queue)
                timer.schedule(deadline: .now() + 5, repeating: 5)
                timer.setEventHandler { [weak self] in
                    guard let self = self, !self.stopped else { return }
                    if self.pendingWrites.values.contains(where: { Date().timeIntervalSince($0) > 15 }) { self.fail(TunnelError.timeout); return }
                    if !self.ready {
                        if Date().timeIntervalSince(self.started) > 15 { self.fail(TunnelError.timeout) }
                    } else if Date().timeIntervalSince(self.lastReceived) > 70 {
                        self.fail(TunnelError.timeout)
                    } else { self.send(4, Data()) {} }
                }
                self.timer = timer; timer.resume()
                conn.start(queue: self.queue)
            } catch { self.fail(error) }
        }
    }
    private func hello(_ p: TunnelProfile) {
        do {
            let data = try JSONSerialization.data(withJSONObject: ["version": 1, "device": p.device, "token": p.token])
            send(1, data) { self.receiveFrame { kind, payload in
                do {
                    guard kind == 2,
                          let json = try JSONSerialization.jsonObject(with: payload) as? [String: Any],
                          json["version"] as? Int == 1, json["mtu"] as? Int == 1280,
                          IPv4Address(json["ipv4"] as? String ?? "") == IPv4Address(p.ipv4),
                          IPv6Address(json["ipv6"] as? String ?? "") == IPv6Address(p.ipv6) else { throw TunnelError.invalid }
                    self.configure(p)
                } catch { self.fail(error) }
            } }
        } catch { fail(error) }
    }
    private func configure(_ p: TunnelProfile) {
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: p.host)
        settings.mtu = 1280
        let v4 = NEIPv4Settings(addresses: [p.ipv4], subnetMasks: ["255.255.255.0"])
        let v6 = NEIPv6Settings(addresses: [p.ipv6], networkPrefixLengths: [64])
        var r4: [NEIPv4Route] = []; var r6: [NEIPv6Route] = []
        for route in p.routes {
            let parts = route.split(separator: "/"); let prefix = Int(parts[1])!
            if IPv4Address(String(parts[0])) != nil {
                let mask: UInt32 = prefix == 0 ? 0 : UInt32.max << (32 - prefix)
                let text = [24,16,8,0].map { String((mask >> $0) & 255) }.joined(separator: ".")
                r4.append(NEIPv4Route(destinationAddress: String(parts[0]), subnetMask: text))
            } else { r6.append(NEIPv6Route(destinationAddress: String(parts[0]), networkPrefixLength: NSNumber(value: prefix))) }
        }
        v4.includedRoutes = r4; v6.includedRoutes = r6
        if IPv4Address(p.host) != nil { v4.excludedRoutes = [NEIPv4Route(destinationAddress: p.host, subnetMask: "255.255.255.255")] }
        else { v6.excludedRoutes = [NEIPv6Route(destinationAddress: p.host, networkPrefixLength: 128)] }
        settings.ipv4Settings = v4; settings.ipv6Settings = v6
        let dns = NEDNSSettings(servers: p.dns); dns.matchDomains = [""]
        settings.dnsSettings = dns
        setTunnelNetworkSettings(settings) { error in self.queue.async {
            guard !self.stopped else { return }
            if let error = error { self.fail(error); return }
            self.ready = true
            self.completion?(nil); self.completion = nil
            self.readPackets(); self.readRemote()
        } }
    }
    private func readPackets() {
        guard !stopped else { return }
        packetFlow.readPackets { packets, _ in self.queue.async { self.sendPackets(packets, 0) } }
    }
    private func sendPackets(_ packets: [Data], _ index: Int) {
        guard !stopped else { return }
        if index == packets.count { readPackets(); return }
        guard validPacket(packets[index], source: true) else { sendPackets(packets, index + 1); return }
        send(3, packets[index]) { self.sendPackets(packets, index + 1) }
    }
    private func readRemote() {
        receiveFrame { kind, packet in
            if kind == 3 {
                guard self.validPacket(packet, source: false) else { self.fail(TunnelError.packet); return }
                self.packetFlow.writePackets([packet], withProtocols: [NSNumber(value: packet[0] >> 4 == 4 ? AF_INET : AF_INET6)])
            } else if kind != 5 || !packet.isEmpty { self.fail(TunnelError.invalid); return }
            self.readRemote()
        }
    }
    private func validPacket(_ packet: Data, source: Bool) -> Bool {
        guard let p = profile, (20...1280).contains(packet.count) else { return false }
        let b = [UInt8](packet); let offset: Int; let address: Data
        if b[0] >> 4 == 4 {
            guard Int(b[0] & 15) * 4 >= 20, Int(b[0] & 15) * 4 <= b.count,
                  Int(b[2]) * 256 + Int(b[3]) == b.count else { return false }
            offset = source ? 12 : 16; address = IPv4Address(p.ipv4)!.rawValue
        } else {
            guard b[0] >> 4 == 6, b.count >= 40, Int(b[4]) * 256 + Int(b[5]) + 40 == b.count else { return false }
            offset = source ? 8 : 24; address = IPv6Address(p.ipv6)!.rawValue
        }
        return Data(b[offset..<(offset + address.count)]) == address
    }
    private func send(_ kind: UInt8, _ payload: Data, _ done: @escaping () -> Void) {
        guard !stopped else { return }
        guard payload.count < 4096 else { fail(TunnelError.invalid); return }
        var len = UInt32(payload.count + 1).bigEndian
        var data = withUnsafeBytes(of: &len) { Data($0) }; data.append(kind); data.append(payload)
        guard pendingWrites.count < 16 else { fail(TunnelError.connection); return }
        writeID &+= 1
        let id = writeID; pendingWrites[id] = Date()
        connection?.send(content: data, completion: .contentProcessed { error in
            self.pendingWrites.removeValue(forKey: id)
            guard !self.stopped else { return }
            if let error = error { self.fail(error) } else { done() }
        })
    }
    private func exact(_ count: Int, _ done: @escaping (Data) -> Void) {
        guard !stopped else { return }
        connection?.receive(minimumIncompleteLength: count, maximumLength: count) { data, _, complete, error in
            guard !self.stopped else { return }
            guard error == nil, !complete, let data = data, data.count == count else { self.fail(TunnelError.connection); return }
            done(data)
        }
    }
    private func receiveFrame(_ done: @escaping (UInt8, Data) -> Void) {
        exact(4) { data in
            let n = data.reduce(0) { ($0 << 8) | Int($1) }
            guard (1...4096).contains(n) else { self.fail(TunnelError.invalid); return }
            self.exact(n) { body in
                self.lastReceived = Date()
                done(body[0], Data(body.dropFirst()))
            }
        }
    }
    private func fail(_ error: Error) {
        guard !stopped else { return }
        stopped = true; timer?.cancel(); connection?.cancel()
        if let completion = completion { self.completion = nil; completion(error) }
        else { cancelTunnelWithError(error) }
        profile = nil
    }
    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        queue.async {
            self.stopped = true; self.timer?.cancel(); self.connection?.cancel()
            self.completion?(TunnelError.connection); self.completion = nil; self.profile = nil
            completionHandler()
        }
    }
}
