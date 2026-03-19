import Foundation

// FSKit filesystem stub.
// Phase 0 will determine the correct FSKit subclass
// (FSUnaryFileSystem vs FSBlockDeviceFileSystem).
// For now, this is a placeholder to verify compilation and linkage.

public enum MNFTSFileSystem {
    public static let name = "mNFTS"
    public static let version = "0.1.0"
}
