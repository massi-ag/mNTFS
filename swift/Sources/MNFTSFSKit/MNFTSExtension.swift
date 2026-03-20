// TODO: FSKit requires a System Extension target with proper entitlements
// and code signing. The `import FSKit` line will not compile under plain
// Swift Package Manager builds. Once an Xcode project with a System
// Extension target is set up, uncomment the import and superclass.
//
// import FSKit
import Foundation

/// mNFTS FSKit filesystem extension.
/// Subclasses FSUnaryFileSystem to handle NTFS volumes.
///
/// This extension is loaded by fskitd when an NTFS partition is detected.
/// It delegates all filesystem operations to libmnfts via the FFI bridge.
///
/// Note: This requires System Extension packaging with proper entitlements
/// and code signing to be loaded by FSKit. During development, the
/// MNFTSFileSystem class can be used directly for testing.
// @objc
// final class MNFTSExtension: FSUnaryFileSystem {
//
//     override class var isMainApp: Bool { false }
//
//     // FSUnaryFileSystemOperations conformance will be added
//     // when the System Extension build target is set up.
//     // The operations delegate to MNFTSVolume which uses FFIBridge.
// }

/// Placeholder so the file compiles under SPM while the real
/// FSUnaryFileSystem subclass is commented out above.
enum MNFTSExtensionPlaceholder {
    static let note = "Enable FSKit extension via Xcode System Extension target"
}
