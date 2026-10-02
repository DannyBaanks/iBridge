import Foundation

struct BootstrapPayload: Codable {
    let schema: String
    let deviceUdid: String
    let deviceName: String
    let deviceVersion: String
    let appleId: String
    let anisetteServer: String
    let pairingPath: String
    let secretPath: String
    let createdAt: String
}

struct AccountSecretPayload: Codable {
    let schema: String
    let appleId: String
    let password: String
}

struct MobileConfiguration {
    let appleID: String
    let anisetteServer: String
    let deviceUDID: String
    let deviceName: String
    let deviceVersion: String

    static func load() -> MobileConfiguration? {
        let defaults = UserDefaults.standard
        guard
            let appleID = defaults.string(forKey: "bootstrap.appleID"),
            let anisetteServer = defaults.string(forKey: "bootstrap.anisetteServer"),
            let deviceUDID = defaults.string(forKey: "bootstrap.deviceUDID"),
            let deviceName = defaults.string(forKey: "bootstrap.deviceName"),
            let deviceVersion = defaults.string(forKey: "bootstrap.deviceVersion")
        else { return nil }
        return MobileConfiguration(
            appleID: appleID,
            anisetteServer: anisetteServer,
            deviceUDID: deviceUDID,
            deviceName: deviceName,
            deviceVersion: deviceVersion
        )
    }
}

enum BootstrapImporter {
    static let pairingAccount = "devicePairing"
    static let passwordAccount = "applePassword"

    static func importIfPresent() throws {
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let directory = documents.appendingPathComponent("iBridgeBootstrap", isDirectory: true)
        let payloadURL = directory.appendingPathComponent("bootstrap.json")
        guard FileManager.default.fileExists(atPath: payloadURL.path) else { return }

        let payload = try JSONDecoder().decode(BootstrapPayload.self, from: Data(contentsOf: payloadURL))
        guard payload.schema == "ibridge.mobile-bootstrap/1" else {
            throw NSError(
                domain: "iBridgeBootstrap",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Unsupported bootstrap schema"]
            )
        }

        let pairingURL = documents.appendingPathComponent(payload.pairingPath)
        let secretURL = documents.appendingPathComponent(payload.secretPath)
        let pairing = try Data(contentsOf: pairingURL)
        let secret = try JSONDecoder().decode(AccountSecretPayload.self, from: Data(contentsOf: secretURL))

        guard secret.schema == "ibridge.mobile-account-secret/1" else {
            throw NSError(
                domain: "iBridgeBootstrap",
                code: 2,
                userInfo: [NSLocalizedDescriptionKey: "Unsupported account secret schema"]
            )
        }
        guard secret.appleId.caseInsensitiveCompare(payload.appleId) == .orderedSame else {
            throw NSError(
                domain: "iBridgeBootstrap",
                code: 3,
                userInfo: [NSLocalizedDescriptionKey: "Bootstrap account identity did not match"]
            )
        }

        try KeychainStore.set(pairing, account: pairingAccount)
        try KeychainStore.setString(secret.password, account: passwordAccount)

        let defaults = UserDefaults.standard
        defaults.set(payload.appleId, forKey: "bootstrap.appleID")
        defaults.set(payload.anisetteServer, forKey: "bootstrap.anisetteServer")
        defaults.set(payload.deviceUdid, forKey: "bootstrap.deviceUDID")
        defaults.set(payload.deviceName, forKey: "bootstrap.deviceName")
        defaults.set(payload.deviceVersion, forKey: "bootstrap.deviceVersion")
        defaults.set(payload.createdAt, forKey: "bootstrap.createdAt")

        try? FileManager.default.removeItem(at: secretURL)
        try? FileManager.default.removeItem(at: pairingURL)
        try? FileManager.default.removeItem(at: payloadURL)
        try? FileManager.default.removeItem(at: directory)
    }
}
