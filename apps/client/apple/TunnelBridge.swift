import Foundation
import NetworkExtension
#if os(macOS)
import FlutterMacOS
#else
import Flutter
#endif

final class TunnelBridge {
    private var manager: NETunnelProviderManager?
    private var busy = false
    private let channel: FlutterMethodChannel
    init(messenger: FlutterBinaryMessenger) {
        channel = FlutterMethodChannel(name: "org.routeloom/vpn", binaryMessenger: messenger)
        channel.setMethodCallHandler { [weak self] call, result in self?.handle(call, result) }
    }
    private var providerID: String { (Bundle.main.bundleIdentifier ?? "org.routeloom.routeloom") + ".PacketTunnel" }
    // A stalled/restarted extension must not leave Flutter polling permanently in flight.
    // Completion and timeout run on the main queue and deliver exactly one result.
    private func request(_ session: NETunnelProviderSession, _ message: String, _ done: @escaping (Data?) -> Void) {
        var completed = false
        let finish: (Data?) -> Void = { data in
            DispatchQueue.main.async {
                guard !completed else { return }
                completed = true
                done(data.flatMap { $0.count <= 1_048_576 ? $0 : nil })
            }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 4) { finish(nil) }
        do { try session.sendProviderMessage(Data(message.utf8), responseHandler: finish) }
        catch { finish(nil) }
    }
    private func load(_ done: @escaping (NETunnelProviderManager?, Error?) -> Void) {
        NETunnelProviderManager.loadAllFromPreferences { managers, error in
            DispatchQueue.main.async {
                let manager = managers?.first { ($0.protocolConfiguration as? NETunnelProviderProtocol)?.providerBundleIdentifier == self.providerID }
                self.manager = manager
                done(manager, error)
            }
        }
    }
    private func handle(_ call: FlutterMethodCall, _ result: @escaping FlutterResult) {
        if call.method == "snapshot" || call.method == "monitorCommand" {
            let snapshot = call.method == "snapshot"
            let command = call.arguments as? String ?? ""
            guard snapshot || command == "clear" || command == "diagnose" else { result(false); return }
            load { manager, error in
                // sendProviderMessage can launch a stopped extension. Reading the dashboard must not.
                guard error == nil, let session = manager?.connection as? NETunnelProviderSession,
                      session.status == .connected || session.status == .reasserting else {
                    if snapshot { result(nil) } else { result(false) }; return
                }
                self.request(session, snapshot ? "snapshot" : "monitor:" + command) { data in
                    let text = data.flatMap { String(data: $0, encoding: .utf8) }
                    if snapshot { result(text) } else { result(text == "ok") }
                }
            }
        } else if call.method == "status" {
            load { manager, error in
                if error != nil { result("failed"); return }
                switch manager?.connection.status {
                case .connected, .reasserting:
                    if let session = manager?.connection as? NETunnelProviderSession {
                        self.request(session, "status") { data in result(data.flatMap { String(data:$0,encoding:.utf8) } ?? "reconnecting") }
                    } else { result("reconnecting") }
                case .connecting: result("connecting")
                case .disconnecting: result("disconnecting")
                case .invalid: result("unavailable")
                default: result("disconnected")
                }
            }
        } else if call.method == "forgetProfile" {
            guard let raw = call.arguments as? String, let data = raw.data(using: .utf8), let requested = try? TunnelProfile.parse(data) else { self.finish(result,TunnelError.invalid); return }
            guard let savedData = try? BackgroundProfile.load(), let saved = try? TunnelProfile.parse(savedData), saved.device == requested.device, saved.endpoint == requested.endpoint else { result(nil); return }
            load { manager, error in
                guard error == nil else { self.finish(result,error); return }
                if let status = manager?.connection.status, status == .connected || status == .connecting || status == .reasserting || status == .disconnecting { self.finish(result,TunnelError.connection); return }
                let clear = { (error: Error?) in
                    if let error = error { self.finish(result,error); return }
                    do { try BackgroundProfile.clear(); self.finish(result,nil) } catch { self.finish(result,error) }
                }
                if let manager = manager { manager.removeFromPreferences(completionHandler: clear) } else { clear(nil) }
            }
        } else if call.method == "guardStatus" {
            load { manager, _ in result((manager?.protocolConfiguration as? NETunnelProviderProtocol)?.includeAllNetworks == true) }
        } else if call.method == "unlock" {
            load { manager, error in
                guard let manager = manager, error == nil else { result(nil); return }
                manager.isOnDemandEnabled = false
                (manager.protocolConfiguration as? NETunnelProviderProtocol)?.includeAllNetworks = false
                manager.saveToPreferences { error in
                    if error == nil { manager.connection.stopVPNTunnel() }
                    self.finish(result,error)
                }
            }
        } else if call.method == "disconnect" {
            load { manager, _ in
                if (manager?.protocolConfiguration as? NETunnelProviderProtocol)?.includeAllNetworks == true,
                   let session = manager?.connection as? NETunnelProviderSession {
                    do { try session.sendProviderMessage(Data("pause".utf8)) { _ in result(nil) } }
                    catch { self.finish(result,error) }
                } else {
                    manager?.isOnDemandEnabled = false
                    manager?.saveToPreferences { _ in manager?.connection.stopVPNTunnel() }
                    result(nil)
                }
            }
        } else if call.method == "connect" {
            guard !busy, let raw = call.arguments as? String, let data = raw.data(using: .utf8),
                  let profile = try? TunnelProfile.parse(data) else {
                result(FlutterError(code: "profile", message: "Invalid profile or operation in progress", details: nil)); return
            }
            busy = true
            load { existing, error in
                guard error == nil else { self.finish(result, error); return }
                if let status = existing?.connection.status, status == .connected || status == .connecting || status == .reasserting || status == .disconnecting {
                    // On Demand may race a mode/profile edit. Never silently resume old routing rules.
                    guard let saved = try? BackgroundProfile.load(), saved == data else { self.finish(result,TunnelError.connection); return }
                    if let session = existing?.connection as? NETunnelProviderSession {
                        do { try session.sendProviderMessage(Data("resume".utf8)) { _ in self.finish(result,nil) } }
                        catch { self.finish(result,error) }
                    } else { self.finish(result,TunnelError.connection) }; return
                }
                let manager = existing ?? NETunnelProviderManager()
                let configuration = NETunnelProviderProtocol()
                configuration.providerBundleIdentifier = self.providerID
                configuration.serverAddress = profile.server_name
                do { try BackgroundProfile.save(data) } catch { self.finish(result,error); return }
                configuration.includeAllNetworks = profile.options?.kill_switch == true
                configuration.excludeLocalNetworks = false
                let onDemand = NEOnDemandRuleConnect(); onDemand.interfaceTypeMatch = .any
                manager.onDemandRules = [onDemand]
                manager.isOnDemandEnabled = profile.options?.kill_switch == true || (profile.options?.auto_reconnect ?? true)
                // Only the shared Keychain contains the background profile.
                manager.protocolConfiguration = configuration
                manager.localizedDescription = "RouteLoom"
                manager.isEnabled = true
                manager.saveToPreferences { error in
                    guard error == nil else { self.finish(result,error); return }
                    manager.loadFromPreferences { error in
                        guard error == nil else { self.finish(result,error); return }
                        do {
                            // Start options are visible in macOS extended VPN status.
                            // The provider reads the profile from the shared Keychain only.
                            try manager.connection.startVPNTunnel()
                            self.manager = manager; self.finish(result,nil)
                        } catch { self.finish(result,error) }
                    }
                }
            }
        } else { result(FlutterMethodNotImplemented) }
    }
    private func finish(_ result: @escaping FlutterResult, _ error: Error?) {
        DispatchQueue.main.async {
            self.busy = false
            if error != nil { result(FlutterError(code: "vpn", message: "VPN configuration or connection failed", details: nil)) }
            else { result(nil) }
        }
    }
}
