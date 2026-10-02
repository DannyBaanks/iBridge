import Foundation

@MainActor
final class MobileModel: ObservableObject {
    @Published private(set) var configuration: MobileConfiguration?
    @Published private(set) var busy = false
    @Published var status = "Preparando iBridge…"
    @Published var needsCredential = false
    @Published var needsTwoFactor = false
    @Published var verificationCode = ""

    private var pendingIPA: URL?
    private let selfReleaseURL = URL(string: "https://github.com/DannyBaanks/iBridge/releases/latest/download/iBridge-Mobile.ipa")!

    init() {
        do {
            try BootstrapImporter.importIfPresent()
            configuration = MobileConfiguration.load()
            needsCredential = KeychainStore.string(account: "applePassword") == nil
            status = configuration == nil
                ? "Instala iBridge desde iLoader Desktop una vez para completar el pairing."
                : "Listo para firmar."
        } catch {
            status = "Bootstrap falló: \(error.localizedDescription)"
        }
    }

    func saveCredential(_ value: String) {
        guard !value.isEmpty else {
            status = "Escribe la credencial de tu Apple ID."
            return
        }
        do {
            try KeychainStore.setString(value, account: "applePassword")
            needsCredential = false
            status = "Cuenta guardada en Keychain."
        } catch {
            status = "Keychain falló: \(error.localizedDescription)"
        }
    }

    func installImportedIPA(_ source: URL) async {
        guard let copied = copyIntoManagedStorage(source) else { return }
        await signAndInstall(copied, verificationCode: nil)
    }

    func submitTwoFactor() async {
        guard let pendingIPA else { return }
        let code = verificationCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !code.isEmpty else { return }
        needsTwoFactor = false
        await signAndInstall(pendingIPA, verificationCode: code)
    }

    func refreshAll() async {
        guard !busy else { return }
        let paths = UserDefaults.standard.stringArray(forKey: "managedIPAPaths") ?? []
        for path in paths {
            let url = URL(fileURLWithPath: path)
            guard FileManager.default.fileExists(atPath: path) else { continue }
            await signAndInstall(url, verificationCode: nil)
            if needsTwoFactor { return }
        }
        await refreshSelf()
    }

    func refreshSelf() async {
        guard !busy else { return }
        do {
            status = "Descargando iBridge Mobile…"
            let (downloaded, _) = try await URLSession.shared.download(from: selfReleaseURL)
            let cache = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("iBridge-Mobile.ipa")
            try? FileManager.default.removeItem(at: cache)
            try FileManager.default.moveItem(at: downloaded, to: cache)
            await signAndInstall(cache, verificationCode: nil)
        } catch {
            status = "No pude descargar iBridge Mobile: \(error.localizedDescription)"
        }
    }

    private func signAndInstall(_ ipa: URL, verificationCode: String?) async {
        guard let configuration else {
            status = "Falta el bootstrap del iPhone."
            return
        }
        guard KeychainStore.string(account: "applePassword") != nil else {
            needsCredential = true
            status = "Conecta tu Apple ID una vez."
            return
        }
        guard let pairing = KeychainStore.data(account: "devicePairing") else {
            status = "Falta el pairing. Reinstala iBridge desde Desktop."
            return
        }
        guard let address = DeviceAddress.current() else {
            status = "No encontré una interfaz local para este iPhone."
            return
        }

        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("iBridge", isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: appSupport, withIntermediateDirectories: true)
        } catch {
            status = "No pude crear el almacenamiento de firma: \(error.localizedDescription)"
            return
        }

        busy = true
        pendingIPA = ipa
        status = "Firmando e instalando \(ipa.lastPathComponent)…"
        defer { busy = false }

        do {
            let response = try await CoreBridge.signAndInstall(CoreRequest(
                appleId: configuration.appleID,
                verificationCode: verificationCode,
                anisetteServer: configuration.anisetteServer,
                inputIpa: ipa.path,
                storageDir: appSupport.path,
                pairingBase64: pairing.base64EncodedString(),
                deviceAddress: address,
                scopeId: nil
            ))

            if response.ok {
                pendingIPA = nil
                self.verificationCode = ""
                needsTwoFactor = false
                status = "Firmada e instalada ✓"
            } else if response.code == "two_factor_required" {
                needsTwoFactor = true
                status = "Apple pidió el código 2FA."
            } else if response.code == "credential_missing" {
                needsCredential = true
                status = "Vuelve a conectar tu Apple ID."
            } else {
                status = "\(response.code): \(response.message)"
            }
        } catch {
            status = "Falló la operación: \(error.localizedDescription)"
        }
    }

    private func copyIntoManagedStorage(_ source: URL) -> URL? {
        let accessing = source.startAccessingSecurityScopedResource()
        defer { if accessing { source.stopAccessingSecurityScopedResource() } }

        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let directory = appSupport.appendingPathComponent("ManagedIPAs", isDirectory: true)
        let destination = directory.appendingPathComponent(source.lastPathComponent)
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try? FileManager.default.removeItem(at: destination)
            try FileManager.default.copyItem(at: source, to: destination)
            var paths = UserDefaults.standard.stringArray(forKey: "managedIPAPaths") ?? []
            if !paths.contains(destination.path) {
                paths.append(destination.path)
                UserDefaults.standard.set(paths, forKey: "managedIPAPaths")
            }
            return destination
        } catch {
            status = "No pude importar la IPA: \(error.localizedDescription)"
            return nil
        }
    }
}
