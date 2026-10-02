import Foundation
import Darwin

enum DeviceAddress {
    static func current() -> String? {
        var addresses: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&addresses) == 0, let first = addresses else { return nil }
        defer { freeifaddrs(addresses) }

        var fallback: String?
        var pointer: UnsafeMutablePointer<ifaddrs>? = first
        while let interface = pointer?.pointee {
            defer { pointer = interface.ifa_next }
            guard let address = interface.ifa_addr else { continue }
            let family = address.pointee.sa_family
            guard family == UInt8(AF_INET) else { continue }

            let name = String(cString: interface.ifa_name)
            let flags = Int32(interface.ifa_flags)
            guard (flags & IFF_UP) != 0, (flags & IFF_LOOPBACK) == 0 else { continue }

            var hostname = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            let result = getnameinfo(
                address,
                socklen_t(address.pointee.sa_len),
                &hostname,
                socklen_t(hostname.count),
                nil,
                0,
                NI_NUMERICHOST
            )
            guard result == 0 else { continue }
            let value = String(cString: hostname)
            if name == "en0" { return value }
            if fallback == nil { fallback = value }
        }
        return fallback
    }
}
