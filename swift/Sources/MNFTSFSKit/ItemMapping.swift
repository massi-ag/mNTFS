import Foundation

/// Maps between FSKit item IDs and NTFS MFT references.
enum ItemMapping {
    /// NTFS root directory MFT reference number.
    static let rootMFTReference: UInt64 = 5

    /// Convert MFT reference to a stable item identifier (as Data).
    static func itemData(from mftReference: UInt64) -> Data {
        var ref = mftReference
        return Data(bytes: &ref, count: 8)
    }

    /// Extract MFT reference from item identifier data.
    static func mftReference(from data: Data) -> UInt64 {
        guard data.count >= 8 else { return 0 }
        return data.withUnsafeBytes { $0.load(as: UInt64.self) }
    }
}
