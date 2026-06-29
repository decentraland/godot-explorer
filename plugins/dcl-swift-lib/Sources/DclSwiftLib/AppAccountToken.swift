import Foundation
import CryptoKit

/// Deterministic wallet → UUID used as StoreKit's `appAccountToken`.
///
/// MUST stay byte-identical to the backend's `uuidFromWallet` (credits-server)
/// and mobile-bff so the token embedded in Apple's signed JWS maps back to the
/// buyer's wallet. The server re-derives this from the signed-fetch wallet and
/// rejects the purchase (`token_mismatch`) if it doesn't match.
///
/// Algorithm: first 16 bytes of `SHA256("dcl-iap:" + wallet.lowercased())`,
/// taken as a UUID verbatim — NOT an RFC-4122 v5 UUID (the version/variant bits
/// are left exactly as the hash produced them).
///
/// Fixed vectors (see AppAccountTokenTests):
///   0x8cB8e16EA85793c0dA573615248b6c91C88dF2DD -> ce2f25a6-fe1a-5f30-ec9c-ef3c1d75c81c
///   0x0000000000000000000000000000000000000000 -> 44e6abd6-e058-f70f-3570-6d70f81e24d9
enum IapAppAccountToken {
    static func derive(forWallet wallet: String) -> UUID? {
        let normalized = wallet.lowercased()
        guard !normalized.isEmpty else { return nil }
        let hash = SHA256.hash(data: Data(("dcl-iap:" + normalized).utf8))
        let bytes = Array(hash.prefix(16))
        guard bytes.count == 16 else { return nil }
        return UUID(uuid: (
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15]
        ))
    }
}
