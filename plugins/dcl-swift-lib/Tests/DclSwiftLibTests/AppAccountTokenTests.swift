import XCTest
@testable import DclSwiftLib

final class AppAccountTokenTests: XCTestCase {
    /// Fixed vectors shared across iOS / mobile-bff / credits-server. If any of
    /// these change, the wallet ↔ appAccountToken mapping has broken and the
    /// server will reject purchases with `token_mismatch`.
    func testKnownVectors() {
        let cases: [(wallet: String, expected: String)] = [
            ("0x8cB8e16EA85793c0dA573615248b6c91C88dF2DD", "ce2f25a6-fe1a-5f30-ec9c-ef3c1d75c81c"),
            ("0x0000000000000000000000000000000000000000", "44e6abd6-e058-f70f-3570-6d70f81e24d9"),
        ]
        for c in cases {
            let token = IapAppAccountToken.derive(forWallet: c.wallet)
            XCTAssertEqual(token?.uuidString.lowercased(), c.expected, "wallet \(c.wallet)")
        }
    }

    /// The wallet is lowercased before hashing, so casing must not change the token.
    func testCaseInsensitive() {
        let mixed = IapAppAccountToken.derive(forWallet: "0x8cB8e16EA85793c0dA573615248b6c91C88dF2DD")
        let lower = IapAppAccountToken.derive(forWallet: "0x8cb8e16ea85793c0da573615248b6c91c88df2dd")
        XCTAssertEqual(mixed, lower)
        XCTAssertNotNil(mixed)
    }

    func testEmptyWalletIsNil() {
        XCTAssertNil(IapAppAccountToken.derive(forWallet: ""))
    }
}
