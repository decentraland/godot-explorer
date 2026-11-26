import { useCallback, useEffect, useState } from 'react';
import type { BackpackApi, Catalog } from './api';

export type CatalogState =
  | { status: 'loading' }
  | { status: 'error' }
  | ({ status: 'ready' } & Catalog);

/** Loads the catalog through the api; `retry` re-runs a failed load. */
export function useCatalog(api: BackpackApi): {
  catalog: CatalogState;
  retry: () => void;
} {
  const [attempt, setAttempt] = useState(0);
  const [catalog, setCatalog] = useState<CatalogState>({ status: 'loading' });

  useEffect(() => {
    let cancelled = false;
    setCatalog({ status: 'loading' });
    api
      .loadCatalog()
      .then(loaded => {
        if (cancelled) {
          return;
        }
        setCatalog({ status: 'ready', ...loaded });
        // Owned items arrive after the base ones; without them the base catalog still works.
        api
          .loadOwned()
          .then(owned => {
            if (!cancelled) {
              setCatalog({ status: 'ready', ...withOwned(loaded, owned) });
            }
          })
          .catch(() => {});
      })
      .catch(() => {
        if (!cancelled) {
          setCatalog({ status: 'error' });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [api, attempt]);

  const retry = useCallback(() => setAttempt(count => count + 1), []);
  return { catalog, retry };
}

/** The catalog plus the owned items it does not list yet. */
export function withOwned(catalog: Catalog, owned: Catalog): Catalog {
  const add = <T extends { id: string }>(base: T[], more: T[]): T[] => {
    const known = new Set(base.map(item => item.id.toLowerCase()));
    return [...base, ...more.filter(item => !known.has(item.id.toLowerCase()))];
  };
  return {
    wearables: add(catalog.wearables, owned.wearables),
    emotes: add(catalog.emotes, owned.emotes),
  };
}
