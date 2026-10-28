import { useCallback, useEffect, useState } from 'react';
import { DiscoverData, fetchDiscover } from './api';

export type DiscoverState =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'ready'; data: DiscoverData };

let cached: DiscoverData | null = null;

export function useDiscover(): { state: DiscoverState; retry: () => void } {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<DiscoverState>(
    cached ? { status: 'ready', data: cached } : { status: 'loading' },
  );

  useEffect(() => {
    if (cached && attempt === 0) {
      return undefined;
    }
    let cancelled = false;
    setState({ status: 'loading' });
    fetchDiscover()
      .then(data => {
        cached = data;
        if (!cancelled) {
          setState({ status: 'ready', data });
        }
      })
      .catch(() => !cancelled && setState({ status: 'error' }));
    return () => {
      cancelled = true;
    };
  }, [attempt]);

  const retry = useCallback(() => setAttempt(count => count + 1), []);
  return { state, retry };
}
