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
        // Pinned to a specific commit on the `barebone-split` branch so builds
        // stay reproducible even if the upstream branch is force-pushed or moved.
        .package(
            url: "https://github.com/migueldeicaza/SwiftGodot",
            revision: "f60a71fd22f932f3eed2626e2282386f9ce7d14a"
        ),
    ],
    targets: [
        .target(
            name: "DclSwiftLib",
            dependencies: [
                .product(name: "SwiftGodotRuntimeStatic", package: "SwiftGodot"),
            ]
        ),
        .testTarget(
            name: "DclSwiftLibTests",
            dependencies: ["DclSwiftLib"]
        ),
    ]
)
