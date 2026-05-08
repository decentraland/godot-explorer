import { createHash } from "node:crypto";

// Deterministic wallet → UUID mapping used for StoreKit's `appAccountToken`
// field. The client computes this same UUID and hands it to StoreKit at
// purchase time; Apple includes it inside the signed JWS, and the backend
// re-derives it from the wallet param to prove the buyer actually intended
// to credit that wallet.
//
// One-way (sha256), so the server can VERIFY (wallet → token → match) but
// not REVERSE (token → wallet without knowing the wallet). The client always
// sends the wallet alongside the JWS; this helper just validates the link.
//
// Salt is fixed and public — its only purpose is namespacing so that the
// same wallet doesn't yield the same UUID across unrelated apps.
const SALT = "dcl-iap:";

export function uuidFromWallet(walletAddress: string): string {
  const hash = createHash("sha256")
    .update(SALT + walletAddress.toLowerCase())
    .digest();
  return formatUuid(hash.subarray(0, 16));
}

export function walletMatchesToken(walletAddress: string, appAccountToken: string): boolean {
  return uuidFromWallet(walletAddress) === appAccountToken.toLowerCase();
}

function formatUuid(bytes: Buffer): string {
  // 8-4-4-4-12 hex format. We don't bother setting the version/variant bits;
  // StoreKit accepts any well-formed UUID and we only ever compare for
  // equality, never parse semantically.
  const hex = bytes.toString("hex");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join("-");
}
