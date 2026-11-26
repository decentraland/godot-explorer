import { destinationsUrl, parsePlaces, Place, rows } from './api';

const PAGE = 100;
const WINDOW_MS = 7 * 24 * 60 * 60 * 1000;

type Raw = Record<string, unknown>;

// The BFF's timestamps are UTC; read to the second.
const within = (raw: Raw | undefined, field: string, now: number) => {
  const at = raw?.[field];
  return (
    typeof at === 'string' &&
    at !== '' &&
    now - Date.parse(`${at.slice(0, 19)}Z`) <= WINDOW_MS
  );
};

const num = (value: unknown) => (typeof value === 'number' ? value : 0);
const byActivity = (a: Raw, b: Raw) =>
  num(b.user_count) - num(a.user_count) ||
  num(b.like_score) - num(a.like_score);

/**
 * Destinations created or redeployed in the last 7 days, the busiest first. The API sorts by one
 * column and has no deployed_at order, so this joins the newest ones (paged while the window lasts)
 * with the most active ones deployed in it; redeployed scenes nobody is in are left out.
 */
export async function fetchNewInDecentraland(
  get: (url: string) => Promise<unknown>,
  now = Date.now(),
): Promise<Place[]> {
  const page = async (query: string) =>
    rows(await get(destinationsUrl(`${query}&limit=${PAGE}`)));
  const found: Raw[] = [];
  const seen = new Set<unknown>();
  let offset = 0;
  let more = true;
  while (more) {
    const items = await page(`offset=${offset}&order_by=created_at&order=desc`);
    // Item by item: on iOS the BFF returns places then worlds, each sorted on its own.
    for (const raw of items.filter(item => within(item, 'created_at', now))) {
      seen.add(raw.id);
      found.push(raw);
    }
    more =
      items.length >= PAGE &&
      within(items[items.length - 1], 'created_at', now);
    // Not `+= PAGE`: on iOS the BFF can ignore `limit` and return everything from the offset on.
    offset += items.length;
  }
  const active = await page('offset=0&order_by=most_active');
  found.push(
    ...active.filter(
      raw => !seen.has(raw.id) && within(raw, 'deployed_at', now),
    ),
  );
  return parsePlaces({ data: found.sort(byActivity) });
}
