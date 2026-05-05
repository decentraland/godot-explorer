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
    ],
    targets: [
        .target(
            name: "DclSwiftLib",
            dependencies: [
                .product(name: "SwiftGodotRuntimeStatic", package: "SwiftGodot"),
            ]
        ),
    ]
)
