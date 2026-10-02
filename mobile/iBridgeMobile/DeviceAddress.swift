import Foundation
import Darwin

enum DeviceAddress {
    static func current() -> String? {
        var addresses: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&addresses) == 0, let first = addresses else { return nil }
        defer { freeifaddrs(addresses) }

        var pointer: UnsafeMutablePointer<ifaddrs>? = first
        while let interface = pointer?.pointee {
            defer { pointer = interface.ifa_next }

            let flags = Int32(interface.ifa_flags)
            guard (flags & IFF_UP) != 0 else { continue }

            // LocalDevVPN and equivalent loopback setups expose the phone as the
            // peer of a point-to-point utun interface. Reading the peer instead
            // of hard-coding 10.7.0.1 also survives custom/iOS 26.x subnets.
            if (flags & IFF_POINTOPOINT) != 0,
               let destination = interface.ifa_dstaddr,
               let host = numericHost(destination),
               host != "0.0.0.0",
               host != "::" {
                return host
            }
        }

        return nil
    }

    private static func numericHost(_ address: UnsafePointer<sockaddr>) -> String? {
        let family = Int32(address.pointee.sa_family)
        guard family == AF_INET || family == AF_INET6 else { return nil }

        var hostname = [CChar](repeating: 0, count: Int(NI_MAXHOST))
        let length: socklen_t
        if family == AF_INET6 {
            length = socklen_t(MemoryLayout<sockaddr_in6>.size)
        } else {
            length = socklen_t(MemoryLayout<sockaddr_in>.size)
        }

        let result = getnameinfo(
            address,
            length,
            &hostname,
            socklen_t(hostname.count),
            nil,
            0,
            NI_NUMERICHOST
        )
        guard result == 0 else { return nil }
        return String(cString: hostname)
    }
}
