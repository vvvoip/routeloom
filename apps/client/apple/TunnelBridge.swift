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
        if call.method == "status" {
            load { manager, error in
                if error != nil { result("failed"); return }
                switch manager?.connection.status {
                case .connected: result("connected")
                case .connecting, .reasserting: result("connecting")
                case .disconnecting: result("disconnecting")
                case .invalid: result("unavailable")
                default: result("disconnected")
                }
            }
        } else if call.method == "disconnect" {
            load { manager, _ in manager?.connection.stopVPNTunnel(); result(nil) }
        } else if call.method == "connect" {
            guard !busy, let raw = call.arguments as? String, let data = raw.data(using: .utf8),
                  let profile = try? TunnelProfile.parse(data) else {
                result(FlutterError(code: "profile", message: "Invalid profile or operation in progress", details: nil)); return
            }
            busy = true
            load { existing, error in
                guard error == nil else { self.finish(result, error); return }
                if let status = existing?.connection.status, status == .connected || status == .connecting || status == .disconnecting {
                    self.finish(result, TunnelError.connection); return
                }
                let manager = existing ?? NETunnelProviderManager()
                let configuration = NETunnelProviderProtocol()
                configuration.providerBundleIdentifier = self.providerID
                configuration.serverAddress = profile.server_name
                // Credentials are passed only to the running provider, never saved in VPN preferences.
                manager.protocolConfiguration = configuration
                manager.localizedDescription = "RouteLoom"
                manager.isEnabled = true
                manager.saveToPreferences { error in
                    guard error == nil else { self.finish(result,error); return }
                    manager.loadFromPreferences { error in
                        guard error == nil else { self.finish(result,error); return }
                        do {
                            try manager.connection.startVPNTunnel(options: ["profile": raw as NSString])
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
