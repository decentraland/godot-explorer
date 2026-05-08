import Database from "better-sqlite3";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

const DB_PATH = resolve(process.cwd(), "data/iap.db");
mkdirSync(dirname(DB_PATH), { recursive: true });

export const db = new Database(DB_PATH);
db.pragma("journal_mode = WAL");
db.pragma("foreign_keys = ON");

db.exec(`
  CREATE TABLE IF NOT EXISTS iap_transactions (
    original_transaction_id TEXT PRIMARY KEY,
    transaction_id          TEXT NOT NULL,
    wallet_address          TEXT NOT NULL,
    product_id              TEXT NOT NULL,
    credits_granted         INTEGER NOT NULL,
    environment             TEXT NOT NULL,
    bundle_id               TEXT NOT NULL,
    purchase_date_ms        INTEGER NOT NULL,
    granted_at              INTEGER NOT NULL,
    jws                     TEXT NOT NULL,
    refunded_at             INTEGER
  );

  CREATE INDEX IF NOT EXISTS idx_tx_wallet ON iap_transactions(wallet_address);

  CREATE TABLE IF NOT EXISTS user_balance (
    wallet_address TEXT PRIMARY KEY,
    credits        INTEGER NOT NULL DEFAULT 0,
    updated_at     INTEGER NOT NULL
  );

  CREATE TABLE IF NOT EXISTS apple_notifications (
    notification_uuid TEXT PRIMARY KEY,
    notification_type TEXT NOT NULL,
    subtype           TEXT,
    received_at       INTEGER NOT NULL,
    raw_payload       TEXT NOT NULL
  );
`);

export interface TransactionRow {
  original_transaction_id: string;
  transaction_id: string;
  wallet_address: string;
  product_id: string;
  credits_granted: number;
  environment: string;
  bundle_id: string;
  purchase_date_ms: number;
  granted_at: number;
  jws: string;
  refunded_at: number | null;
}

const findTxStmt = db.prepare<[string], TransactionRow>(
  "SELECT * FROM iap_transactions WHERE original_transaction_id = ?",
);
const insertTxStmt = db.prepare(
  `INSERT INTO iap_transactions
     (original_transaction_id, transaction_id, wallet_address, product_id,
      credits_granted, environment, bundle_id, purchase_date_ms, granted_at, jws)
   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
);
const upsertBalanceStmt = db.prepare(
  `INSERT INTO user_balance (wallet_address, credits, updated_at)
   VALUES (?, ?, ?)
   ON CONFLICT(wallet_address) DO UPDATE SET
     credits = credits + excluded.credits,
     updated_at = excluded.updated_at`,
);
const getBalanceStmt = db.prepare<[string], { credits: number }>(
  "SELECT credits FROM user_balance WHERE wallet_address = ?",
);
const markRefundedStmt = db.prepare(
  "UPDATE iap_transactions SET refunded_at = ? WHERE original_transaction_id = ? AND refunded_at IS NULL",
);
const decrementBalanceStmt = db.prepare(
  `UPDATE user_balance
     SET credits = MAX(0, credits - ?), updated_at = ?
     WHERE wallet_address = ?`,
);

export function findTransaction(originalId: string): TransactionRow | undefined {
  return findTxStmt.get(originalId);
}

export function getBalance(walletAddress: string): number {
  const row = getBalanceStmt.get(walletAddress);
  return row?.credits ?? 0;
}

interface GrantInput {
  originalTransactionId: string;
  transactionId: string;
  walletAddress: string;
  productId: string;
  credits: number;
  environment: string;
  bundleId: string;
  purchaseDateMs: number;
  jws: string;
}

export function grantTransactionAtomically(input: GrantInput): {
  granted: boolean;
  balance: number;
} {
  const now = Date.now();
  const tx = db.transaction((data: GrantInput) => {
    const existing = findTxStmt.get(data.originalTransactionId);
    if (existing) {
      return { granted: false, balance: getBalanceStmt.get(data.walletAddress)?.credits ?? 0 };
    }
    insertTxStmt.run(
      data.originalTransactionId,
      data.transactionId,
      data.walletAddress,
      data.productId,
      data.credits,
      data.environment,
      data.bundleId,
      data.purchaseDateMs,
      now,
      data.jws,
    );
    upsertBalanceStmt.run(data.walletAddress, data.credits, now);
    return { granted: true, balance: getBalanceStmt.get(data.walletAddress)?.credits ?? 0 };
  });
  return tx(input);
}

export function applyRefundAtomically(originalId: string): {
  applied: boolean;
  walletAddress: string | null;
} {
  const now = Date.now();
  const tx = db.transaction(() => {
    const row = findTxStmt.get(originalId);
    if (!row) return { applied: false, walletAddress: null };
    if (row.refunded_at !== null) return { applied: false, walletAddress: row.wallet_address };
    const result = markRefundedStmt.run(now, originalId);
    if (result.changes === 0) return { applied: false, walletAddress: row.wallet_address };
    decrementBalanceStmt.run(row.credits_granted, now, row.wallet_address);
    return { applied: true, walletAddress: row.wallet_address };
  });
  return tx();
}

const insertNotificationStmt = db.prepare(
  `INSERT OR IGNORE INTO apple_notifications
     (notification_uuid, notification_type, subtype, received_at, raw_payload)
   VALUES (?, ?, ?, ?, ?)`,
);

export function recordNotification(uuid: string, type: string, subtype: string | null, raw: string): boolean {
  const result = insertNotificationStmt.run(uuid, type, subtype, Date.now(), raw);
  return result.changes > 0;
}
