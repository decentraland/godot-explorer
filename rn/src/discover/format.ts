const MINUTE = 60_000;
const HOUR = 60 * MINUTE;

/** "IN 5 MINS", "IN 2 HOURS", or a short date for anything further than a day out. */
export function startsIn(startsAt: number, now: number): string {
  const delta = startsAt - now;
  if (delta < MINUTE) {
    return 'STARTING';
  }
  if (delta < HOUR) {
    const mins = Math.round(delta / MINUTE);
    return `IN ${mins} ${mins === 1 ? 'MIN' : 'MINS'}`;
  }
  if (delta < 24 * HOUR) {
    const hours = Math.round(delta / HOUR);
    return `IN ${hours} ${hours === 1 ? 'HOUR' : 'HOURS'}`;
  }
  return new Date(startsAt)
    .toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
    .toUpperCase();
}

export const formatRate = (rate: number | null): string =>
  rate === null ? '—' : `${Math.round(rate * 100)}%`;

export const formatUsers = (count: number): string =>
  String(count).padStart(3, '0');

export const matchesQuery = (title: string, query: string): boolean =>
  title.toLowerCase().includes(query.trim().toLowerCase());
