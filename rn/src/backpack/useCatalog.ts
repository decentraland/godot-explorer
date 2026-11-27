import { useCallback, useEffect, useState } from 'react';
import { getBackpackApi } from './api';
import type { BackpackApi, Catalog } from './api';

export type CatalogState =
  | { status: 'loading' }
  | { status: 'error' }
  | ({ status: 'ready' } & Catalog);

/** The last catalog each api loaded: the Backpack opens on it and refreshes it behind. */
const last = new WeakMap<BackpackApi, Catalog>();
const pending = new WeakMap<BackpackApi, Promise<Catalog>>();

/**
 * The catalog with what the account owns, in one piece. Shown base-first, the owned items landed a
 * moment later and the equipped ones jumped to the front of All, moving every tile. Without the
 * owned list the base catalog still works.
 */
function loadFull(api: BackpackApi): Promise<Catalog> {
  let load = pending.get(api);
  if (!load) {
    load = api
      .loadCatalog()
      .then(base =>
        api.loadOwned().then(
          owned => withOwned(base, owned),
          () => base,
        ),
      )
      .then(
        full => {
          pending.delete(api);
          last.set(api, full);
          return full;
        },
        error => {
          pending.delete(api);
          throw error;
        },
      );
    pending.set(api, load);
  }
  return load;
}

/** Loads the catalog (and what the account owns) ahead of the Backpack, so it opens ready. */
export function preloadCatalog(api: BackpackApi = getBackpackApi()): void {
  loadFull(api).catch(() => {});
}

/** Loads the catalog through the api; `retry` re-runs a failed load. */
export function useCatalog(api: BackpackApi): {
  catalog: CatalogState;
  retry: () => void;
} {
  const [attempt, setAttempt] = useState(0);
  const [catalog, setCatalog] = useState<CatalogState>(() => {
    const known = last.get(api);
    return known ? { status: 'ready', ...known } : { status: 'loading' };
  });

  useEffect(() => {
    let cancelled = false;
    if (!last.has(api)) {
      setCatalog({ status: 'loading' });
    }
    loadFull(api)
      .then(full => {
        if (!cancelled) {
          setCatalog({ status: 'ready', ...full });
        }
      })
      .catch(() => {
        // A failed refresh keeps the catalog already on screen.
        if (!cancelled && !last.has(api)) {
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
