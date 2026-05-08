import { Hono } from "hono";
import { z } from "zod";

import {
  applyRefundAtomically,
  findTransaction,
  getBalance,
  grantTransactionAtomically,
  recordNotification,
} from "./db.ts";
import { decodeNotificationJws, decodeTransactionJws, JwsRejected } from "./apple.ts";
import { creditsFor } from "./products.ts";
import { uuidFromWallet, walletMatchesToken } from "./wallet.ts";

const grantBody = z.object({
  jws: z.string().min(20),
  // The wallet to credit. Authority binding is enforced via the JWS's
  // appAccountToken, not via request signing — anyone can post here, but the
  // JWS Apple signed must declare an appAccountToken == uuidFromWallet(this).
  walletAddress: z.string().regex(/^0x[a-fA-F0-9]{40}$/),
});

const webhookBody = z.object({
  signedPayload: z.string().min(20),
});

export const app = new Hono();

app.get("/", (c) => c.json({ ok: true, service: "dcl-iap-backend" }));

app.get("/balance/:wallet", (c) => {
  const wallet = c.req.param("wallet").toLowerCase();
  if (!/^0x[a-f0-9]{40}$/.test(wallet)) {
    return c.json({ error: "invalid wallet" }, 400);
  }
  return c.json({ wallet, credits: getBalance(wallet) });
});

app.post("/iap/grant", async (c) => {
  const parsed = grantBody.safeParse(await c.req.json().catch(() => ({})));
  if (!parsed.success) {
    return c.json({ error: "invalid body", details: parsed.error.issues }, 400);
  }
  const { jws, walletAddress } = parsed.data;
  const wallet = walletAddress.toLowerCase();

  let tx;
  try {
    tx = await decodeTransactionJws(jws);
  } catch (e) {
    if (e instanceof JwsRejected) {
      return c.json({ error: "jws_rejected", reason: e.message }, 400);
    }
    throw e;
  }

  // appAccountToken is mandatory: it's the cryptographic link between Apple's
  // signed receipt and the wallet we're about to credit. Without it (or if it
  // doesn't match) anyone with a captured JWS could redirect credits to any
  // wallet.
  if (!tx.appAccountToken) {
    return c.json({ error: "missing_app_account_token" }, 400);
  }
  if (!walletMatchesToken(wallet, tx.appAccountToken)) {
    return c.json(
      {
        error: "wallet_token_mismatch",
        expected: uuidFromWallet(wallet),
        got: tx.appAccountToken,
      },
      403,
    );
  }

  const credits = creditsFor(tx.productId);
  if (credits <= 0) {
    return c.json({ error: "unknown_product", productId: tx.productId }, 400);
  }

  const result = grantTransactionAtomically({
    originalTransactionId: tx.originalTransactionId,
    transactionId: tx.transactionId,
    walletAddress: wallet,
    productId: tx.productId,
    credits,
    environment: tx.environment,
    bundleId: tx.bundleId,
    purchaseDateMs: tx.purchaseDate,
    jws,
  });

  return c.json({
    status: result.granted ? "granted" : "already_processed",
    productId: tx.productId,
    creditsGranted: result.granted ? credits : 0,
    balance: result.balance,
    environment: tx.environment,
  });
});

app.post("/apple/webhook", async (c) => {
  const parsed = webhookBody.safeParse(await c.req.json().catch(() => ({})));
  if (!parsed.success) {
    return c.json({ error: "invalid body" }, 400);
  }
  const { signedPayload } = parsed.data;

  let notif;
  try {
    notif = await decodeNotificationJws(signedPayload);
  } catch (e) {
    if (e instanceof JwsRejected) {
      return c.json({ error: "rejected", reason: e.message }, 400);
    }
    throw e;
  }

  recordNotification(
    notif.notificationUUID,
    notif.notificationType,
    notif.subtype ?? null,
    signedPayload,
  );

  if (notif.notificationType === "REFUND" && notif.data.signedTransactionInfo) {
    let refundedTx;
    try {
      refundedTx = await decodeTransactionJws(notif.data.signedTransactionInfo);
    } catch (e) {
      if (e instanceof JwsRejected) {
        return c.json({ error: "refund_tx_rejected", reason: e.message }, 400);
      }
      throw e;
    }
    const existing = findTransaction(refundedTx.originalTransactionId);
    if (!existing) {
      return c.json({
        status: "refund_unknown_tx",
        originalId: refundedTx.originalTransactionId,
      });
    }
    const result = applyRefundAtomically(refundedTx.originalTransactionId);
    return c.json({
      status: result.applied ? "refund_applied" : "refund_already_processed",
      originalId: refundedTx.originalTransactionId,
      walletAddress: result.walletAddress,
    });
  }

  return c.json({ status: "ack", notificationType: notif.notificationType });
});
