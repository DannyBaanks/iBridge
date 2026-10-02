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
    static func importIfPresent() throws {
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let directory = documents.appendingPathComponent("iBridgeBootstrap", isDirectory: true)
        let payloadURL = directory.appendingPathComponent("bootstrap.json")
        guard FileManager.default.fileExists(atPath: payloadURL.path) else { return }

        let payload = try JSONDecoder().decode(BootstrapPayload.self, from: Data(contentsOf: payloadURL))
        guard payload.schema == "ibridge.mobile-bootstrap/1" else {
            throw NSError(domain: "iBridgeBootstrap", code: 1, userInfo: [NSLocalizedDescriptionKey: "Unsupported bootstrap schema"])
        }

        let pairingURL = documents.appendingPathComponent(payload.pairingPath)
        let pairing = try Data(contentsOf: pairingURL)
        try KeychainStore.set(pairing, account: "devicePairing")

        let defaults = UserDefaults.standard
        defaults.set(payload.appleId, forKey: "bootstrap.appleID")
        defaults.set(payload.anisetteServer, forKey: "bootstrap.anisetteServer")
        defaults.set(payload.deviceUdid, forKey: "bootstrap.deviceUDID")
        defaults.set(payload.deviceName, forKey: "bootstrap.deviceName")
        defaults.set(payload.deviceVersion, forKey: "bootstrap.deviceVersion")
        defaults.set(payload.createdAt, forKey: "bootstrap.createdAt")

        try? FileManager.default.removeItem(at: pairingURL)
        try? FileManager.default.removeItem(at: payloadURL)
        try? FileManager.default.removeItem(at: directory)
    }
}
