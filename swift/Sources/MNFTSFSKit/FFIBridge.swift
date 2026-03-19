import CLibMNFTS
import Foundation

enum MNFTSBridge {
    static func ping() -> Bool {
        return mnfts_ping() == OK
    }

    static func version() -> String {
        guard let cStr = mnfts_version() else { return "unknown" }
        return String(cString: cStr)
    }
}
