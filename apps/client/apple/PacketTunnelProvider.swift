import NetworkExtension
import Network
import Security
@_silgen_name("routeloom_start") private func engineStart(_ data: UnsafePointer<UInt8>, _ count: Int, _ protect: UnsafeRawPointer?, _ context: UInt) -> UInt64
@_silgen_name("routeloom_write") private func engineWrite(_ handle: UInt64, _ data: UnsafePointer<UInt8>, _ count: Int) -> Bool
@_silgen_name("routeloom_read") private func engineRead(_ handle: UInt64, _ data: UnsafeMutablePointer<UInt8>, _ capacity: Int) -> Int
@_silgen_name("routeloom_status") private func engineStatus(_ handle: UInt64) -> UInt8
@_silgen_name("routeloom_stop") private func engineStop(_ handle: UInt64)
final class PacketTunnelProvider: NEPacketTunnelProvider {
    private let queue = DispatchQueue(label: "org.routeloom.packet")
    private let outputQueue = DispatchQueue(label: "org.routeloom.packet.output")
    private var handle: UInt64 = 0
    private var stopped = false
    private var paused = false
    private var raw = Data()
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        queue.async {
            do {
                let data = (options?["profile"] as? String)?.data(using: .utf8) ?? (try? BackgroundProfile.load())
                guard let data = data else { throw TunnelError.invalid }
                let p = try TunnelProfile.parse(data)
                self.raw = data
                self.handle = data.withUnsafeBytes { engineStart($0.bindMemory(to: UInt8.self).baseAddress!, data.count, nil, 0) }
                guard self.handle != 0 else { throw TunnelError.invalid }
                self.setTunnelNetworkSettings(self.settings(p)) { error in self.queue.async {
                    if let error = error { engineStop(self.handle); self.handle = 0; completionHandler(error); return }
                    self.reasserting = true
                    completionHandler(nil); self.readPackets(); self.poll()
                } }
            } catch { completionHandler(error) }
        }
    }
    private func settings(_ p: TunnelProfile) -> NEPacketTunnelNetworkSettings {
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: p.host)
        settings.mtu = 1280
        let v4 = NEIPv4Settings(addresses: [p.ipv4], subnetMasks: ["255.255.255.0"])
        let v6 = NEIPv6Settings(addresses: [p.ipv6], networkPrefixLengths: [64])
        var r4: [NEIPv4Route] = []; var r6: [NEIPv6Route] = []
        for route in p.routes + ["198.18.0.0/16","fdfe::/112"] {
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
        return settings
    }

    private func readPackets() {
        guard !stopped else { return }
        packetFlow.readPackets { packets, _ in self.queue.async {
            if !self.paused && self.handle != 0 {
                for packet in packets where packet.count <= 1280 {
                    packet.withUnsafeBytes { bytes in
                        if let base = bytes.bindMemory(to: UInt8.self).baseAddress { _ = engineWrite(self.handle, base, packet.count) }
                    }
                }
            }
            self.readPackets()
        } }
    }
    private func poll() {
        guard !stopped else { return }
        guard !paused && handle != 0 else {
            queue.asyncAfter(deadline: .now() + 0.2) { self.poll() }; return
        }
        let active = handle
        // Waiting for output must not hold up packetFlow input or control messages.
        outputQueue.async {
            var buffer = [UInt8](repeating: 0, count: 1280)
            let count = engineRead(active, &buffer, 1280)
            let packet = count > 0 ? Data(buffer.prefix(count)) : nil
            self.queue.async {
                guard !self.stopped else { return }
                if !self.paused && self.handle == active {
                    if let packet = packet {
                        self.packetFlow.writePackets([packet], withProtocols: [NSNumber(value: packet[0] >> 4 == 4 ? AF_INET : AF_INET6)])
                    }
                    self.reasserting = engineStatus(active) != 1
                }
                self.queue.asyncAfter(deadline: .now() + 0.001) { self.poll() }
            }
        }
    }
    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        queue.async {
            let message = String(data: messageData, encoding: .utf8)
            if message == "pause" { self.paused = true; engineStop(self.handle); self.handle = 0; self.reasserting = false }
            if message == "resume" && self.paused {
                self.handle = self.raw.withUnsafeBytes { engineStart($0.bindMemory(to: UInt8.self).baseAddress!,self.raw.count,nil,0) }
                self.paused = false
            }
            let status = self.paused ? "blocked" : (engineStatus(self.handle) == 1 ? "connected" : "reconnecting")
            completionHandler?(status.data(using: .utf8))
        }
    }
    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        queue.async { self.stopped = true; engineStop(self.handle); self.handle = 0; completionHandler() }
    }
}
