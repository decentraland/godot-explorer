import { useEffect } from 'react';
import { AppState } from 'react-native';
import { core } from '../core';

/** How often the core is told the app is alive (it writes the liveness mark and closes the open's
 * settle window). */
export const TICK_MS = 5000;

const quietly = (call: () => void) => {
  try {
    call();
  } catch {
    // Analytics may not be installed (no-engine builds); the chain is best effort.
  }
};

/** Reports launch, foreground, background and a heartbeat for the `App Opened` event. */
export function useSessionTracking() {
  useEffect(() => {
    quietly(() => core().sessionLaunched());
    const tick = setInterval(
      () => quietly(() => core().sessionTick()),
      TICK_MS,
    );
    const sub = AppState.addEventListener('change', state => {
      if (state === 'active') {
        quietly(() => core().sessionForegrounded());
      } else if (state === 'background') {
        quietly(() => core().sessionBackgrounded());
      }
    });
    return () => {
      clearInterval(tick);
      sub.remove();
    };
  }, []);
}
