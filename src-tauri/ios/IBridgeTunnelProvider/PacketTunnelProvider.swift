// iBridge local packet reflector.
//
// Based on the packet-reflection technique from SideStore/StosVPN:
// https://github.com/SideStore/StosVPN
// Copyright (c) 2025 SideStore Team.
// See THIRD_PARTY_NOTICES.md for license and attribution.

import NetworkExtension

final class PacketTunnelProvider: NEPacketTunnelProvider {
    private var tunnelDeviceIP = "10.7.0.0"
    private var tunnelFakeIP = "10.7.0.1"
    private let tunnelSubnetMask = "255.255.255.0"

    private var deviceIPValue: UInt32 = 0
    private var fakeIPValue: UInt32 = 0

    override func startTunnel(
        options: [String: NSObject]?,
        completionHandler: @escaping (Error?) -> Void
    ) {
        if let deviceIP = options?["TunnelDeviceIP"] as? String {
            tunnelDeviceIP = deviceIP
        }
        if let fakeIP = options?["TunnelFakeIP"] as? String {
            tunnelFakeIP = fakeIP
        }

        deviceIPValue = ipToUInt32(tunnelDeviceIP)
        fakeIPValue = ipToUInt32(tunnelFakeIP)

        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: tunnelDeviceIP)
        let ipv4 = NEIPv4Settings(
            addresses: [tunnelDeviceIP],
            subnetMasks: [tunnelSubnetMask]
        )
        ipv4.includedRoutes = [
            NEIPv4Route(destinationAddress: tunnelDeviceIP, subnetMask: tunnelSubnetMask)
        ]
        ipv4.excludedRoutes = [.default()]
        settings.ipv4Settings = ipv4

        setTunnelNetworkSettings(settings) { [weak self] error in
            guard error == nil else {
                completionHandler(error)
                return
            }
            self?.pumpPackets()
            completionHandler(nil)
        }
    }

    override func stopTunnel(
        with reason: NEProviderStopReason,
        completionHandler: @escaping () -> Void
    ) {
        completionHandler()
    }

    private func pumpPackets() {
        packetFlow.readPackets { [weak self] packets, protocols in
            guard let self else { return }

            var modified = packets
            for index in modified.indices
            where protocols[index].int32Value == AF_INET && modified[index].count >= 20 {
                modified[index].withUnsafeMutableBytes { bytes in
                    guard let words = bytes.baseAddress?.assumingMemoryBound(to: UInt32.self) else {
                        return
                    }
                    let source = UInt32(bigEndian: words[3])
                    let destination = UInt32(bigEndian: words[4])
                    if source == self.deviceIPValue {
                        words[3] = self.fakeIPValue.bigEndian
                    }
                    if destination == self.fakeIPValue {
                        words[4] = self.deviceIPValue.bigEndian
                    }
                }
            }

            self.packetFlow.writePackets(modified, withProtocols: protocols)
            self.pumpPackets()
        }
    }

    private func ipToUInt32(_ address: String) -> UInt32 {
        let parts = address.split(separator: ".")
        guard parts.count == 4,
              let a = UInt32(parts[0]),
              let b = UInt32(parts[1]),
              let c = UInt32(parts[2]),
              let d = UInt32(parts[3]) else {
            return 0
        }
        return (a << 24) | (b << 16) | (c << 8) | d
    }
}
