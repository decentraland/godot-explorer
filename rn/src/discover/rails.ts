import { useCallback, useEffect, useRef, useState } from 'react';
import type { Page } from './types';

export type RailState<T> =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'ready'; items: T[]; hasMore: boolean };

/** Connection lost: every primary rail failed (an empty list means nothing was asked yet). */
export const isOffline = (rails: RailState<unknown>[]): boolean =>
  rails.length > 0 && rails.every(rail => rail.status === 'error');

/** One rail that loads on its own and can append the next page. */
export function useRail<T>(
  fetchPage: (offset: number) => Promise<Page<T>>,
  deps: unknown[],
) {
  const [state, setState] = useState<RailState<T>>({ status: 'loading' });
  const [more, setMore] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const latest = useRef(state);
  latest.current = state;

  useEffect(() => {
    let cancelled = false;
    setState({ status: 'loading' });
    fetchPage(0)
      .then(
        page =>
          !cancelled &&
          setState({
            status: 'ready',
            items: page.items,
            hasMore: page.hasMore,
          }),
      )
      .catch(() => !cancelled && setState({ status: 'error' }));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, attempt]);

  const loadMore = useCallback(() => {
    const current = latest.current;
    if (current.status !== 'ready' || !current.hasMore || more) {
      return;
    }
    setMore(true);
    fetchPage(current.items.length)
      .then(page =>
        setState({
          status: 'ready',
          items: [...current.items, ...page.items],
          hasMore: page.hasMore,
        }),
      )
      .catch(() => setState({ ...current, hasMore: false }))
      .finally(() => setMore(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [more, ...deps]);

  const reload = useCallback(() => setAttempt(n => n + 1), []);
  return { state, loadMore, loadingMore: more, reload };
}
