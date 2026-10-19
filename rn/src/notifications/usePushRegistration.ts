import { useEffect, useRef } from 'react';
import { AppState } from 'react-native';
import { onCoreEvents } from '../core/events';
import {
  askNotificationPermission,
  registerPush,
  syncDay1Notification,
} from './push';

/** Once the player is signed in: ask for the permission, register, and register again whenever
 * the app comes back (the token can rotate while it was away). The Day 1 welcome follows the same
 * moments, plus the flags landing, signed in or not. */
export function usePushRegistration(signedIn: boolean) {
  const asked = useRef(false);
  useEffect(
    () =>
      onCoreEvents({
        flagsLoaded: () => {
          syncDay1Notification().catch(() => {});
        },
      }),
    [],
  );
  useEffect(() => {
    if (!signedIn) {
      return;
    }
    const register = () => {
      registerPush().catch(() => {});
      syncDay1Notification().catch(() => {});
    };
    (async () => {
      if (!asked.current) {
        asked.current = true;
        await askNotificationPermission().catch(() => false);
      }
      register();
    })();
    const sub = AppState.addEventListener('change', state => {
      if (state === 'active') {
        register();
      }
    });
    return () => sub.remove();
  }, [signedIn]);
}
