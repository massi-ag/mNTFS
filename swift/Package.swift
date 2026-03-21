// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "mNFTS",
    platforms: [.macOS(.v15)],
    products: [
        .executable(name: "mnfts", targets: ["MNFTSCLI"]),
        .executable(name: "mnfts-watchdog", targets: ["MNFTSWatchdog"]),
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
            exclude: ["Info.plist"],
            linkerSettings: [
                .linkedLibrary("libmnfts", .when(platforms: [.macOS])),
                .unsafeFlags(["-L../target/release"], .when(platforms: [.macOS])),
            ]
        ),
        .executableTarget(
            name: "MNFTSCLI",
            dependencies: [
                "CLibMNFTS",
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ],
            linkerSettings: [
                .linkedLibrary("libmnfts", .when(platforms: [.macOS])),
                .unsafeFlags(["-L../target/release"], .when(platforms: [.macOS])),
            ]
        ),
        .executableTarget(
            name: "MNFTSExtension",
            dependencies: ["MNFTSFSKit"],
            exclude: ["Info.plist"],
            linkerSettings: [
                .linkedFramework("FSKit"),
                .linkedLibrary("libmnfts", .when(platforms: [.macOS])),
                .unsafeFlags(["-L../target/release"], .when(platforms: [.macOS])),
            ]
        ),
        .executableTarget(
            name: "MNFTSWatchdog",
            dependencies: [],
            exclude: ["com.mnfts.watchdog.plist"]
        ),
    ]
)
