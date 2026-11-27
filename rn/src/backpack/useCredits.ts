import { useEffect, useState } from 'react';
import { BackpackApi } from './api';

/** The credits balance while `enabled`, or null until it loads (and when it fails). */
export function useCredits(api: BackpackApi, enabled: boolean): number | null {
  const [credits, setCredits] = useState<number | null>(null);
  useEffect(() => {
    if (!enabled) {
      return;
    }
    let live = true;
    api
      .loadCredits()
      .then(amount => live && setCredits(amount))
      .catch(() => live && setCredits(null));
    return () => {
      live = false;
    };
  }, [api, enabled]);
  return enabled ? credits : null;
}
