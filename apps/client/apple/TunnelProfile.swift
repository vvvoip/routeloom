import Foundation
import Network
import Security

struct TunnelProfile: Decodable {
    let endpoint: String
    let server_name: String
    let ca_pem: String
    let device: String
    let token: String
    let ipv4: String
    let ipv6: String
    let routes: [String]
    let dns: [String]
    let options: TunnelOptions?
    var host: String {
        let part = endpoint.prefix(upTo: endpoint.lastIndex(of: ":")!)
        return String(part).trimmingCharacters(in: CharacterSet(charactersIn: "[]"))
    }
    var port: UInt16 { UInt16(endpoint.suffix(from: endpoint.index(after: endpoint.lastIndex(of: ":")!)))! }
    static func parse(_ data: Data) throws -> TunnelProfile {
        guard data.count <= 65536 else { throw TunnelError.invalid }
        let p = try JSONDecoder().decode(Self.self, from: data)
        guard let split = p.endpoint.lastIndex(of: ":"), let port = UInt16(p.endpoint[p.endpoint.index(after: split)...]), port > 0,
              !p.server_name.isEmpty, p.server_name.count <= 253, p.ca_pem.count <= 32768,
              p.device.range(of: "^[A-Za-z0-9_-]{1,64}$", options: .regularExpression) != nil,
              p.token.range(of: "^[0-9a-f]{64}$", options: .regularExpression) != nil,
              IPv4Address(p.ipv4) != nil, IPv6Address(p.ipv6) != nil,
              (1...64).contains(p.routes.count), (1...4).contains(p.dns.count)
        else { throw TunnelError.invalid }
        guard IPv4Address(p.host) != nil || IPv6Address(p.host) != nil else { throw TunnelError.invalid }
        for route in p.routes {
            let parts = route.split(separator: "/")
            guard parts.count == 2, let prefix = Int(parts[1]),
                  (IPv4Address(String(parts[0])) != nil && (0...32).contains(prefix)) ||
                  (IPv6Address(String(parts[0])) != nil && (0...128).contains(prefix)) else { throw TunnelError.invalid }
        }
        for ip in p.dns { guard IPv4Address(ip) != nil || IPv6Address(ip) != nil else { throw TunnelError.invalid } }
        return p
    }
    func anchors() throws -> [SecCertificate] {
        let parts = ca_pem.components(separatedBy: "-----BEGIN CERTIFICATE-----").dropFirst()
        let certs = parts.compactMap { part -> SecCertificate? in
            guard let end = part.range(of: "-----END CERTIFICATE-----") else { return nil }
            let base64 = String(part[..<end.lowerBound]).components(separatedBy: .whitespacesAndNewlines).joined()
            guard let der = Data(base64Encoded: base64) else { return nil }
            return SecCertificateCreateWithData(nil, der as CFData)
        }
        guard !certs.isEmpty, certs.count == parts.count else { throw TunnelError.invalid }
        return certs
    }
}
enum TunnelError: Error { case invalid, connection, timeout, packet }

struct TunnelOptions: Decodable {
    let policy: String?
    let auto_reconnect: Bool?
    let kill_switch: Bool?
    let pac_port: UInt16?
}
enum BackgroundProfile {
    static func query() throws -> [String: Any] {
        guard let group = Bundle.main.object(forInfoDictionaryKey: "RouteLoomKeychainGroup") as? String, !group.isEmpty, !group.contains("$(") else { throw TunnelError.invalid }
        return [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: "RouteLoom.Background.v1", kSecAttrAccount as String: "active", kSecAttrAccessGroup as String: group]
    }
    static func save(_ data: Data) throws {
        let base = try query()
        let values: [String: Any] = [kSecValueData as String: data, kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly]
        let result = SecItemUpdate(base as CFDictionary, values as CFDictionary)
        if result == errSecItemNotFound {
            var item = base; values.forEach { item[$0.key] = $0.value }
            guard SecItemAdd(item as CFDictionary, nil) == errSecSuccess else { throw TunnelError.invalid }
        } else if result != errSecSuccess { throw TunnelError.invalid }
    }
    static func clear() throws {
        let result = SecItemDelete(try query() as CFDictionary)
        guard result == errSecSuccess || result == errSecItemNotFound else { throw TunnelError.invalid }
    }
    static func load() throws -> Data {
        var item = try query(); item[kSecReturnData as String] = true; item[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(item as CFDictionary, &result) == errSecSuccess, let data = result as? Data else { throw TunnelError.invalid }
        return data
    }
}
