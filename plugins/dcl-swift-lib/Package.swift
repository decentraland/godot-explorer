// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "DclSwiftLib",
    platforms: [
        .iOS(.v17),
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "DclSwiftLib",
            type: .dynamic,
            targets: ["DclSwiftLib"]
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/migueldeicaza/SwiftGodot", branch: "barebone-split"),
        .package(url: "https://github.com/reown-com/reown-swift", from: "1.5.0"),
        .package(url: "https://github.com/daltoniam/Starscream", from: "4.0.8"),
        .package(url: "https://github.com/krzyzanowskim/CryptoSwift", from: "1.8.0")
    ],
    targets: [
        .target(
            name: "DclSwiftLib",
            dependencies: [
                .product(name: "SwiftGodotRuntimeStatic", package: "SwiftGodot"),
                .product(name: "WalletConnect", package: "reown-swift"),
                .product(name: "WalletConnectNetworking", package: "reown-swift"),
                .product(name: "Starscream", package: "Starscream"),
                .product(name: "CryptoSwift", package: "CryptoSwift")
            ]
        ),
    ]
)
