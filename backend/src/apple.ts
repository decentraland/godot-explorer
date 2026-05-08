import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import {
  Environment,
  SignedDataVerifier,
} from "@apple/app-store-server-library";
import { decodeJwt } from "jose";

// Source-of-truth payload shapes we read out of Apple-signed JWS. Apple sends
// many more fields; we only validate what we use.

export interface TransactionPayload {
  transactionId: string;
  originalTransactionId: string;
  productId: string;
  bundleId: string;
  environment: string; // "Xcode" | "Sandbox" | "Production"
  purchaseDate: number; // ms epoch
  type: string;
  appAccountToken: string | null;
}

export interface NotificationPayload {
  notificationType: string;
  subtype?: string;
  notificationUUID: string;
  data: {
    bundleId: string;
    environment: string;
    signedTransactionInfo?: string;
    signedRenewalInfo?: string;
  };
}

const ALLOW_UNVERIFIED_XCODE = process.env.ALLOW_UNVERIFIED_XCODE !== "false";
const EXPECTED_BUNDLE_ID =
  process.env.IAP_BUNDLE_ID ?? "org.decentraland.godotexplorer";
const APP_APPLE_ID = process.env.IAP_APP_APPLE_ID
  ? Number(process.env.IAP_APP_APPLE_ID)
  : undefined;
const APPLE_ROOTS_DIR = resolve(process.cwd(), "apple-roots");

export class JwsRejected extends Error {}

function loadAppleRoots(): Buffer[] {
  const files = readdirSync(APPLE_ROOTS_DIR).filter((f) => f.endsWith(".cer"));
  if (files.length === 0) {
    throw new Error(`no Apple root certificates found in ${APPLE_ROOTS_DIR}`);
  }
  return files.map((f) => readFileSync(resolve(APPLE_ROOTS_DIR, f)));
}

const rootCertificates = loadAppleRoots();

// One verifier per environment, lazy-constructed. Apple's lib refuses
// cross-env JWS, so we route based on the env claimed in the unverified
// payload (then re-checked inside verifyAndDecodeTransaction). Production
// requires IAP_APP_APPLE_ID; we don't construct that verifier until a
// production JWS actually arrives.
let verifierSandbox: SignedDataVerifier | null = null;
let verifierProd: SignedDataVerifier | null = null;

function pickVerifier(env: string): SignedDataVerifier | null {
  if (env === "Sandbox") {
    verifierSandbox ??= new SignedDataVerifier(
      rootCertificates,
      /* enableOnlineChecks */ false,
      Environment.SANDBOX,
      EXPECTED_BUNDLE_ID,
    );
    return verifierSandbox;
  }
  if (env === "Production") {
    if (APP_APPLE_ID === undefined) {
      throw new JwsRejected("Production JWS received but IAP_APP_APPLE_ID not configured");
    }
    verifierProd ??= new SignedDataVerifier(
      rootCertificates,
      /* enableOnlineChecks */ false,
      Environment.PRODUCTION,
      EXPECTED_BUNDLE_ID,
      APP_APPLE_ID,
    );
    return verifierProd;
  }
  return null;
}

/**
 * Verify and decode a StoreKit JWS transaction. For Sandbox/Production this
 * does full signature + cert-chain verification via Apple's official library.
 * For Xcode-environment JWS (local StoreKit Configuration File), the JWS is
 * signed by an Xcode-local test cert that is NOT in Apple's chain, so we can
 * only decode the payload — gated by ALLOW_UNVERIFIED_XCODE for safety.
 */
export async function decodeTransactionJws(jws: string): Promise<TransactionPayload> {
  const peeked = decodeJwt(jws) as Record<string, unknown>;
  const environment = String(peeked.environment ?? "");

  let payload: Record<string, unknown>;
  if (environment === "Xcode") {
    if (!ALLOW_UNVERIFIED_XCODE) {
      throw new JwsRejected("Xcode-environment JWS rejected (prod hardening enabled)");
    }
    payload = peeked;
  } else if (environment === "Sandbox" || environment === "Production") {
    const verifier = pickVerifier(environment)!;
    try {
      payload = (await verifier.verifyAndDecodeTransaction(jws)) as unknown as Record<
        string,
        unknown
      >;
    } catch (e) {
      throw new JwsRejected(`Apple verification failed: ${(e as Error).message}`);
    }
  } else {
    throw new JwsRejected(`unknown environment: ${environment}`);
  }

  const bundleId = String(payload.bundleId ?? "");
  if (bundleId !== EXPECTED_BUNDLE_ID) {
    throw new JwsRejected(`bundleId mismatch: ${bundleId}`);
  }

  const transactionId = String(payload.transactionId ?? "");
  const originalTransactionId = String(payload.originalTransactionId ?? "");
  const productId = String(payload.productId ?? "");
  const purchaseDate = Number(payload.purchaseDate ?? 0);
  const type = String(payload.type ?? "");
  if (!transactionId || !originalTransactionId || !productId || !purchaseDate) {
    throw new JwsRejected("transaction JWS missing required fields");
  }

  // appAccountToken is optional in Apple's schema, but we make it MANDATORY
  // — it's how we bind the JWS to the buyer's wallet (see uuidFromWallet).
  const appAccountToken =
    typeof payload.appAccountToken === "string" && payload.appAccountToken.length > 0
      ? payload.appAccountToken.toLowerCase()
      : null;

  return {
    transactionId,
    originalTransactionId,
    productId,
    bundleId,
    environment,
    purchaseDate,
    type,
    appAccountToken,
  };
}

export async function decodeNotificationJws(signedPayload: string): Promise<NotificationPayload> {
  const peeked = decodeJwt(signedPayload) as Record<string, unknown>;
  const data = (peeked.data ?? {}) as Record<string, unknown>;
  const env = String(data.environment ?? "");

  let payload: Record<string, unknown>;
  if (env === "Xcode") {
    if (!ALLOW_UNVERIFIED_XCODE) {
      throw new JwsRejected("Xcode-environment notification rejected (prod hardening enabled)");
    }
    payload = peeked;
  } else if (env === "Sandbox" || env === "Production") {
    const verifier = pickVerifier(env)!;
    try {
      payload = (await verifier.verifyAndDecodeNotification(signedPayload)) as unknown as Record<
        string,
        unknown
      >;
    } catch (e) {
      throw new JwsRejected(`Apple notification verification failed: ${(e as Error).message}`);
    }
  } else {
    throw new JwsRejected(`unknown notification environment: ${env}`);
  }

  const notificationType = String(payload.notificationType ?? "");
  const notificationUUID = String(payload.notificationUUID ?? "");
  const innerData = (payload.data ?? {}) as Record<string, unknown>;
  const bundleId = String(innerData.bundleId ?? "");

  if (!notificationType || !notificationUUID) {
    throw new JwsRejected("notification missing type/uuid");
  }
  if (bundleId !== EXPECTED_BUNDLE_ID) {
    throw new JwsRejected(`notification bundleId mismatch: ${bundleId}`);
  }

  return {
    notificationType,
    subtype: payload.subtype ? String(payload.subtype) : undefined,
    notificationUUID,
    data: {
      bundleId,
      environment: env,
      signedTransactionInfo:
        typeof innerData.signedTransactionInfo === "string"
          ? innerData.signedTransactionInfo
          : undefined,
      signedRenewalInfo:
        typeof innerData.signedRenewalInfo === "string"
          ? innerData.signedRenewalInfo
          : undefined,
    },
  };
}
