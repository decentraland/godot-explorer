import { useEffect, useRef, useState } from 'react';
import {
  addRow,
  expireRows,
  nextExpiry,
  NotificationRow,
  ROW_LIFETIME_MS,
} from './notifications';
import type { ChatLine } from './types';

/** Rows for the click-through list: each new line shows for `lifetime` ms, at most ten at once. */
export function useNotificationRows(
  lines: ChatLine[],
  lifetime: number = ROW_LIFETIME_MS,
  now: () => number = Date.now,
) {
  const [rows, setRows] = useState<NotificationRow[]>([]);
  const lastSeen = useRef<string | null>(null);

  useEffect(() => {
    const after = lastSeen.current
      ? lines.findIndex(l => l.id === lastSeen.current)
      : -1;
    const fresh = lines.slice(after + 1);
    if (fresh.length) {
      lastSeen.current = fresh[fresh.length - 1].id;
      setRows(prev =>
        fresh.reduce((acc, line) => addRow(acc, line, now(), lifetime), prev),
      );
    }
  }, [lines, lifetime, now]);

  useEffect(() => {
    const next = nextExpiry(rows);
    if (next === null || !Number.isFinite(next)) {
      return undefined;
    }
    const timer = setTimeout(
      () => setRows(prev => expireRows(prev, now())),
      Math.max(0, next - now()),
    );
    return () => clearTimeout(timer);
  }, [rows, now]);

  return rows;
}
