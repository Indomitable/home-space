import Foundation
import Security

struct SavedSession: Codable {
    let server: String
    let refreshToken: String
}

/// What the app keeps between runs: the session in the Keychain, the rest in `UserDefaults`.
struct SessionStore: Sendable {
    private static let service = "app.homespace.session"
    private static let account = "session"
    private static let deviceIdKey = "device_id"
    private static let lastServerKey = "last_server"

    /// Identifies this install to the server; survives signing out.
    var deviceId: String {
        if let id = UserDefaults.standard.string(forKey: Self.deviceIdKey) {
            return id
        }
        let id = UUID().uuidString.lowercased()
        UserDefaults.standard.set(id, forKey: Self.deviceIdKey)
        return id
    }

    var lastServer: String? {
        UserDefaults.standard.string(forKey: Self.lastServerKey)
    }

    func loadSession() -> SavedSession? {
        var query = Self.itemQuery
        query[kSecReturnData] = true
        query[kSecMatchLimit] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
            let data = result as? Data
        else { return nil }
        return try? JSONDecoder().decode(SavedSession.self, from: data)
    }

    func saveSession(_ session: SavedSession) {
        UserDefaults.standard.set(session.server, forKey: Self.lastServerKey)
        guard let data = try? JSONEncoder().encode(session) else { return }
        let attributes: [CFString: Any] = [
            kSecValueData: data,
            kSecAttrAccessible: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
        ]
        let status = SecItemUpdate(Self.itemQuery as CFDictionary, attributes as CFDictionary)
        if status == errSecItemNotFound {
            let item = Self.itemQuery.merging(attributes) { _, new in new }
            SecItemAdd(item as CFDictionary, nil)
        }
    }

    func forgetSession() {
        SecItemDelete(Self.itemQuery as CFDictionary)
    }

    private static var itemQuery: [CFString: Any] {
        [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: account,
        ]
    }
}
