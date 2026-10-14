import { useEffect, useReducer } from 'react';
import type { HudApi } from '../hud/hudApi';
import { initialLoading, loadingReducer, SLOW_AFTER_MS } from './loadingModel';

/** Loading phase and progress for the cover; `playing` is the core reaching `InWorld`. */
export function useLoading(api: HudApi, playing: boolean) {
  const [state, dispatch] = useReducer(loadingReducer, initialLoading);

  useEffect(
    () =>
      api.subscribe(event => {
        if (event.type === 'loading_progress') {
          dispatch({ type: 'progress', data: event.data });
        }
      }),
    [api],
  );

  useEffect(() => {
    if (playing) {
      dispatch({ type: 'finished' });
    }
  }, [playing]);

  useEffect(() => {
    if (state.phase !== 'loading') {
      return undefined;
    }
    const timer = setTimeout(() => dispatch({ type: 'slow' }), SLOW_AFTER_MS);
    return () => clearTimeout(timer);
  }, [state.phase]);

  return { state, dispatch };
}
