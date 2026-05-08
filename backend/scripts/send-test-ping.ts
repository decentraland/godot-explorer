// Triggers Apple's "Request a Test Notification" endpoint, which makes Apple
// post a TEST payload to the URL configured in App Store Connect (App Store
// Server Notifications V2). Use this to verify the webhook end-to-end.
//
// Required env vars:
//   ASC_KEY_ID     — App Store Connect API Key ID (10-char alphanumeric)
//   ASC_ISSUER_ID  — top of the Keys page in ASC (UUID)
//   ASC_KEY_PATH   — path to the .p8 private key downloaded from ASC
//   APPLE_ENV      — "sandbox" (default) or "production"
//   IAP_BUNDLE_ID  — defaults to org.decentraland.godotexplorer

import { readFileSync } from "node:fs";
import { SignJWT, importPKCS8 } from "jose";

const KEY_ID = process.env.ASC_KEY_ID;
const ISSUER_ID = process.env.ASC_ISSUER_ID;
const KEY_PATH = process.env.ASC_KEY_PATH;
const BUNDLE_ID = process.env.IAP_BUNDLE_ID ?? "org.decentraland.godotexplorer";
const ENV = (process.env.APPLE_ENV ?? "sandbox").toLowerCase();

if (!KEY_ID || !ISSUER_ID || !KEY_PATH) {
  console.error("Missing env: ASC_KEY_ID, ASC_ISSUER_ID, ASC_KEY_PATH all required");
  process.exit(1);
}

const pem = readFileSync(KEY_PATH, "utf8");
const key = await importPKCS8(pem, "ES256");

const now = Math.floor(Date.now() / 1000);
const jwt = await new SignJWT({ bid: BUNDLE_ID })
  .setProtectedHeader({ alg: "ES256", kid: KEY_ID, typ: "JWT" })
  .setIssuer(ISSUER_ID)
  .setIssuedAt(now)
  .setExpirationTime(now + 1200)
  .setAudience("appstoreconnect-v1")
  .sign(key);

const apiHost =
  ENV === "production"
    ? "https://api.storekit.itunes.apple.com"
    : "https://api.storekit-sandbox.itunes.apple.com";

const url = `${apiHost}/inApps/v1/notifications/test`;
console.log(`POST ${url}`);

const res = await fetch(url, {
  method: "POST",
  headers: {
    Authorization: `Bearer ${jwt}`,
    "Content-Type": "application/json",
  },
});
const text = await res.text();
console.log(`HTTP ${res.status}`);
console.log(text);

if (res.status === 200) {
  try {
    const { testNotificationToken } = JSON.parse(text) as { testNotificationToken: string };
    console.log("\nApple is now posting a TEST notification to your webhook URL.");
    console.log(`Track delivery status with:`);
    console.log(`  GET ${apiHost}/inApps/v1/notifications/test/${testNotificationToken}`);
  } catch {
    /* ignore parse error */
  }
}
