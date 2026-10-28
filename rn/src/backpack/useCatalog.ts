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
        if (!cancelled) {
          setCatalog({ status: 'ready', ...loaded });
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
  }, [api, attempt]);

  const retry = useCallback(() => setAttempt(count => count + 1), []);
  return { catalog, retry };
}
