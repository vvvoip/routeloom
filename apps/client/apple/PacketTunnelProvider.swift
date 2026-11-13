import NetworkExtension
import Network
import Security
import OSLog
@_silgen_name("routeloom_start") private func engineStart(_ data: UnsafePointer<UInt8>, _ count: Int, _ protect: UnsafeRawPointer?, _ context: UInt) -> UInt64
@_silgen_name("routeloom_write") private func engineWrite(_ handle: UInt64, _ data: UnsafePointer<UInt8>, _ count: Int) -> Bool
@_silgen_name("routeloom_read") private func engineRead(_ handle: UInt64, _ data: UnsafeMutablePointer<UInt8>, _ capacity: Int) -> Int
@_silgen_name("routeloom_status") private func engineStatus(_ handle: UInt64) -> UInt8
@_silgen_name("routeloom_stop") private func engineStop(_ handle: UInt64)
@_silgen_name("routeloom_snapshot") private func engineSnapshot(_ handle: UInt64, _ data: UnsafeMutablePointer<UInt8>, _ capacity: Int) -> Int
@_silgen_name("routeloom_command") private func engineCommand(_ handle: UInt64, _ data: UnsafePointer<UInt8>, _ count: Int) -> Bool
final class PacketTunnelProvider: NEPacketTunnelProvider {
    private let logger = Logger(subsystem: "org.routeloom.vpn", category: "lifecycle")
    private let queue = DispatchQueue(label: "org.routeloom.packet")
    private let outputQueue = DispatchQueue(label: "org.routeloom.packet.output")
    private var handle: UInt64 = 0
    private var generation: UInt64 = 0
    private var stopped = false
    private var paused = false
    private var raw = Data()
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        queue.async {
            self.logger.notice("Starting tunnel")
            self.generation &+= 1
            let generation = self.generation
            self.stopped = false; self.paused = false
            if self.handle != 0 { engineStop(self.handle); self.handle = 0 }
            var stage = 1
            do {
                // Never accept credentials through OS-visible start options.
                let data = try BackgroundProfile.load()
                stage = 2
                let p = try TunnelProfile.parse(data)
                self.raw = data
                stage = 3
                self.handle = data.withUnsafeBytes { engineStart($0.bindMemory(to: UInt8.self).baseAddress!, data.count, nil, 0) }
                guard self.handle != 0 else { throw TunnelError.invalid }
                self.setTunnelNetworkSettings(self.settings(p)) { error in self.queue.async {
                    guard !self.stopped && self.generation == generation else { completionHandler(TunnelError.connection); return }
                    if let error = error { engineStop(self.handle); self.handle = 0; completionHandler(self.startupFailure(4, error)); return }
                    self.logger.notice("Tunnel network settings installed")
                    self.reasserting = true
                    completionHandler(nil); self.readPackets(generation); self.poll(generation)
                } }
            } catch { completionHandler(self.startupFailure(stage, error)) }
        }
    }
    private func startupFailure(_ stage: Int, _ error: Error) -> NSError {
        // Only the stage and numeric error code are logged; never a profile or error userInfo.
        let code = (error as NSError).code
        logger.error("Tunnel startup failed at stage \(stage, privacy: .public), error code \(code, privacy: .public)")
        let messages = [1: "Shared Keychain profile unavailable", 2: "Invalid saved profile", 3: "Native engine could not start", 4: "VPN network settings failed"]
        return NSError(domain: "RouteLoom.TunnelStartup", code: stage,
                       userInfo: [NSLocalizedDescriptionKey: messages[stage] ?? "Tunnel startup failed"])
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

    private func readPackets(_ generation: UInt64) {
        guard !stopped && self.generation == generation else { return }
        packetFlow.readPackets { packets, _ in self.queue.async {
            guard !self.stopped && self.generation == generation else { return }
            if !self.paused && self.handle != 0 {
                for packet in packets where packet.count <= 1280 {
                    packet.withUnsafeBytes { bytes in
                        if let base = bytes.bindMemory(to: UInt8.self).baseAddress { _ = engineWrite(self.handle, base, packet.count) }
                    }
                }
            }
            self.readPackets(generation)
        } }
    }
    private func poll(_ generation: UInt64) {
        guard !stopped && self.generation == generation else { return }
        guard !paused && handle != 0 else {
            queue.asyncAfter(deadline: .now() + 0.2) { self.poll(generation) }; return
        }
        let active = handle
        // Waiting for output must not hold up packetFlow input or control messages.
        outputQueue.async {
            var buffer = [UInt8](repeating: 0, count: 1280)
            let count = engineRead(active, &buffer, 1280)
            let packet = count > 0 ? Data(buffer.prefix(count)) : nil
            self.queue.async {
                guard !self.stopped && self.generation == generation else { return }
                if !self.paused && self.handle == active {
                    if let packet = packet {
                        self.packetFlow.writePackets([packet], withProtocols: [NSNumber(value: packet[0] >> 4 == 4 ? AF_INET : AF_INET6)])
                    }
                    self.reasserting = engineStatus(active) == 0
                }
                self.queue.asyncAfter(deadline: .now() + (engineStatus(self.handle) == 2 ? 0.2 : 0.001)) { self.poll(generation) }
            }
        }
    }
    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        queue.async {
            guard messageData.count <= 64, !self.stopped else { completionHandler?(nil); return }
            let message = String(data: messageData, encoding: .utf8)
            if message == "snapshot" {
                guard self.handle != 0, !self.paused else { completionHandler?(nil); return }
                var buffer = [UInt8](repeating: 0, count: 1_048_576)
                let count = engineSnapshot(self.handle, &buffer, 1_048_576)
                completionHandler?(count > 0 && count <= buffer.count ? Data(buffer.prefix(count)) : nil)
                return
            }
            if message == "monitor:clear" || message == "monitor:diagnose" {
                let command = Data(message!.dropFirst("monitor:".count).utf8)
                let accepted = command.withUnsafeBytes {
                    engineCommand(self.handle, $0.bindMemory(to: UInt8.self).baseAddress!, command.count)
                }
                completionHandler?(accepted ? Data("ok".utf8) : nil)
                return
            }
            guard message == "status" || message == "pause" || message == "resume" else { completionHandler?(nil); return }
            if message == "pause" { self.paused = true; engineStop(self.handle); self.handle = 0; self.reasserting = false }
            if message == "resume" && !self.raw.isEmpty && (self.paused || engineStatus(self.handle) == 2) {
                engineStop(self.handle)
                self.handle = self.raw.withUnsafeBytes { engineStart($0.bindMemory(to: UInt8.self).baseAddress!,self.raw.count,nil,0) }
                self.paused = self.handle == 0
            }
            let status = self.paused || engineStatus(self.handle) == 2 ? "blocked" : (engineStatus(self.handle) == 1 ? "connected" : "reconnecting")
            completionHandler?(status.data(using: .utf8))
        }
    }
    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        queue.async {
            self.logger.notice("Stopping tunnel, reason \(reason.rawValue, privacy: .public)")
            self.generation &+= 1; self.stopped = true; engineStop(self.handle); self.handle = 0; self.raw = Data(); completionHandler()
        }
    }
}
