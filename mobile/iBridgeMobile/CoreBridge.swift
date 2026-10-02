import Foundation

struct CoreRequest: Codable, Sendable {
    let appleId: String
    let verificationCode: String?
    let anisetteServer: String
    let inputIpa: String
    let storageDir: String
    let pairingBase64: String
    let deviceAddress: String
    let scopeId: UInt32?
}

struct CoreResponse: Codable, Sendable {
    let ok: Bool
    let code: String
    let message: String
}

enum CoreBridgeError: LocalizedError {
    case encoding
    case emptyResponse
    case invalidResponse(String)

    var errorDescription: String? {
        switch self {
        case .encoding: "No se pudo preparar la solicitud de firma."
        case .emptyResponse: "El core de firma no devolvió respuesta."
        case .invalidResponse(let response): "Respuesta inválida del core: \(response)"
        }
    }
}

enum CoreBridge {
    static func signAndInstall(_ request: CoreRequest) async throws -> CoreResponse {
        try await Task.detached(priority: .userInitiated) {
            let data = try JSONEncoder().encode(request)
            guard let json = String(data: data, encoding: .utf8) else { throw CoreBridgeError.encoding }

            let pointer = json.withCString { ibridge_sign_and_install($0) }
            guard let pointer else { throw CoreBridgeError.emptyResponse }
            defer { ibridge_string_free(pointer) }

            let responseText = String(cString: pointer)
            guard let responseData = responseText.data(using: .utf8) else {
                throw CoreBridgeError.invalidResponse(responseText)
            }
            return try JSONDecoder().decode(CoreResponse.self, from: responseData)
        }.value
    }
}
