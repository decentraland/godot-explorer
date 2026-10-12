import { useCallback, useEffect, useState } from 'react';
import { fetchWearables, Wearable } from './catalog';
import { DEFAULT_EMOTES, Emote, fetchEmoteThumbnails } from './emotes';

export type CatalogState =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'ready'; wearables: Wearable[]; emotes: Emote[] };

let cached: { wearables: Wearable[]; emotes: Emote[] } | null = null;

/** Loads the wearable and emote catalogs once per process; `retry` re-runs a failed load. */
export function useCatalog(): { catalog: CatalogState; retry: () => void } {
  const [attempt, setAttempt] = useState(0);
  const [catalog, setCatalog] = useState<CatalogState>(
    cached ? { status: 'ready', ...cached } : { status: 'loading' },
  );

  useEffect(() => {
    if (cached) {
      return undefined;
    }
    let cancelled = false;
    setCatalog({ status: 'loading' });
    Promise.all([
      fetchWearables(),
      fetchEmoteThumbnails().catch(() => DEFAULT_EMOTES),
    ])
      .then(([wearables, emotes]) => {
        cached = { wearables, emotes };
        if (!cancelled) {
          setCatalog({ status: 'ready', wearables, emotes });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCatalog({ status: 'error' });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [attempt]);

  const retry = useCallback(() => setAttempt(count => count + 1), []);
  return { catalog, retry };
}
