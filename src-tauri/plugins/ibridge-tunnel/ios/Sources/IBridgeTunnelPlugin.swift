import NetworkExtension
import Tauri

private let providerBundleIdentifier = "com.dannybaanks.ibridge.mobile.tunnel"
private let tunnelDeviceIP = "10.7.0.0"
private let tunnelFakeIP = "10.7.0.1"

private func statusName(_ status: NEVPNStatus) -> String {
  switch status {
  case .invalid: return "invalid"
  case .disconnected: return "disconnected"
  case .connecting: return "connecting"
  case .connected: return "connected"
  case .reasserting: return "reasserting"
  case .disconnecting: return "disconnecting"
  @unknown default: return "unknown"
  }
}

private func response(_ manager: NETunnelProviderManager?) -> [String: Any] {
  let status = manager?.connection.status ?? .invalid
  return [
    "connected": status == .connected,
    "status": statusName(status),
  ]
}

private func loadManager(_ completion: @escaping (Result<NETunnelProviderManager, Error>) -> Void) {
  NETunnelProviderManager.loadAllFromPreferences { managers, error in
    if let error {
      completion(.failure(error))
      return
    }

    if let existing = managers?.first(where: {
      ($0.protocolConfiguration as? NETunnelProviderProtocol)?.providerBundleIdentifier
        == providerBundleIdentifier
    }) {
      completion(.success(existing))
      return
    }

    let manager = NETunnelProviderManager()
    let configuration = NETunnelProviderProtocol()
    configuration.providerBundleIdentifier = providerBundleIdentifier
    configuration.serverAddress = "iBridge Local Tunnel"
    manager.protocolConfiguration = configuration
    manager.localizedDescription = "iBridge Local Tunnel"
    manager.isEnabled = true

    manager.saveToPreferences { saveError in
      if let saveError {
        completion(.failure(saveError))
        return
      }
      manager.loadFromPreferences { loadError in
        if let loadError {
          completion(.failure(loadError))
        } else {
          completion(.success(manager))
        }
      }
    }
  }
}

final class IBridgeTunnelPlugin: Plugin {
  @objc public func start(_ invoke: Invoke) throws {
    loadManager { result in
      switch result {
      case .failure(let error):
        invoke.reject(error.localizedDescription)
      case .success(let manager):
        do {
          try manager.connection.startVPNTunnel(options: [
            "TunnelDeviceIP": tunnelDeviceIP as NSString,
            "TunnelFakeIP": tunnelFakeIP as NSString,
          ])
          invoke.resolve(response(manager))
        } catch {
          invoke.reject(error.localizedDescription)
        }
      }
    }
  }

  @objc public func status(_ invoke: Invoke) throws {
    loadManager { result in
      switch result {
      case .failure(let error):
        invoke.reject(error.localizedDescription)
      case .success(let manager):
        invoke.resolve(response(manager))
      }
    }
  }

  @objc public func stop(_ invoke: Invoke) throws {
    loadManager { result in
      switch result {
      case .failure(let error):
        invoke.reject(error.localizedDescription)
      case .success(let manager):
        manager.connection.stopVPNTunnel()
        invoke.resolve(response(manager))
      }
    }
  }
}

@_cdecl("init_plugin_ibridge_tunnel")
func initPlugin() -> Plugin {
  IBridgeTunnelPlugin()
}
