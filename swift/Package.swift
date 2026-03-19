// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "mNFTS",
    platforms: [.macOS(.v15)],
    products: [
        .executable(name: "mnfts", targets: ["MNFTSCLI"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.5.0"),
    ],
    targets: [
        .systemLibrary(
            name: "CLibMNFTS",
            path: "Sources/CLibMNFTS",
            pkgConfig: nil,
            providers: nil
        ),
        .target(
            name: "MNFTSFSKit",
            dependencies: ["CLibMNFTS"],
            linkerSettings: [
                .linkedLibrary("mnfts", .when(platforms: [.macOS])),
                .unsafeFlags(["-L../../target/release"], .when(platforms: [.macOS])),
            ]
        ),
        .executableTarget(
            name: "MNFTSCLI",
            dependencies: [
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ]
        ),
    ]
)
