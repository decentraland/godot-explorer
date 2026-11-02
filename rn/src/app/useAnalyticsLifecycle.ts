import { useEffect } from 'react';
import { AppState } from 'react-native';
import { FLUSH_APP_S, FLUSH_ONBOARDING_S } from '../analytics';
import { core } from '../core';

/** Ships pending analytics when the app goes to the background and keeps the flush interval
 * that fits the phase (onboarding or the app proper) when it comes back. */
export function useAnalyticsLifecycle(entered: boolean) {
  useEffect(() => {
    const seconds = entered ? FLUSH_APP_S : FLUSH_ONBOARDING_S;
    core().setAnalyticsFlushInterval(seconds);
    const subscription = AppState.addEventListener('change', status => {
      if (status === 'background') {
        // The core keeps what it could not ship for the next flush.
        core()
          .flushAnalytics()
          .catch(() => undefined);
      } else if (status === 'active') {
        core().setAnalyticsFlushInterval(seconds);
      }
    });
    return () => subscription.remove();
  }, [entered]);
}
