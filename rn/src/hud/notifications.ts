import type { ChatLine } from './types';

export const MAX_ROWS = 10;
export const ROW_LIFETIME_MS = 6500;

export type NotificationRow = { line: ChatLine; expiresAt: number };

export function addRow(
  rows: NotificationRow[],
  line: ChatLine,
  now: number,
  lifetime: number = ROW_LIFETIME_MS,
): NotificationRow[] {
  return [...rows, { line, expiresAt: now + lifetime }].slice(-MAX_ROWS);
}

export function expireRows(
  rows: NotificationRow[],
  now: number,
): NotificationRow[] {
  const alive = rows.filter(row => row.expiresAt > now);
  return alive.length === rows.length ? rows : alive;
}

export function nextExpiry(rows: NotificationRow[]): number | null {
  return rows.length ? Math.min(...rows.map(row => row.expiresAt)) : null;
}
