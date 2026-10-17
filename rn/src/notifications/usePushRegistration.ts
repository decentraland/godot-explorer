import { useEffect, useRef } from 'react';
import { AppState } from 'react-native';
import { askNotificationPermission, registerPush } from './push';

/** Once the player is signed in: ask for the permission, register, and register again whenever
 * the app comes back (the token can rotate while it was away). */
export function usePushRegistration(signedIn: boolean) {
  const asked = useRef(false);
  useEffect(() => {
    if (!signedIn) {
      return;
    }
    const register = () => registerPush().catch(() => {});
    (async () => {
      if (!asked.current) {
        asked.current = true;
        await askNotificationPermission().catch(() => false);
      }
      await register();
    })();
    const sub = AppState.addEventListener('change', state => {
      if (state === 'active') {
        register();
      }
    });
    return () => sub.remove();
  }, [signedIn]);
}
