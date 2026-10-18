import type { CoreApi } from '../api';
import { Day1Notification, Quality, State } from '../native';
import { createMockAvatar } from './avatar';
import { createMockAvatarApi } from './avatarApi';
import { createMockControl } from './control';
import { createMockFtue } from './ftue';
import { createMockBus } from './emitter';
import { createMockGates } from './gates';
import { createMockPlaces } from './places';
import { createMockProfile } from './profile';
import { createMockScene } from './scene';
import { createMockSession } from './session';
import { createMockSettings } from './settings';

/**
 * Implements the same interface as the native binding, for builds without the engine library. All
 * slices share one app state and one set of listeners, so entering a world reaches `useCoreState`.
 */
export function createMockCore(options: { signedIn?: boolean } = {}): CoreApi {
  const bus = createMockBus(
    options.signedIn ? new State.SignedIn() : new State.SignedOut(),
  );
  const avatar = createMockAvatar();
  return {
    ...createMockSession(bus),
    ...createMockAvatarApi(avatar),
    ...createMockControl(bus, avatar),
    ...createMockSettings(),
    ...createMockFtue(),
    ...createMockProfile(),
    ...createMockPlaces(),
    setPushRegistration: () => {},
    day1Notification: () => new Day1Notification.Keep(),
    sessionLaunched: () => {},
    sessionForegrounded: () => {},
    sessionBackgrounded: () => {},
    sessionTick: () => {},
    reviewCheck: () => false,
    reviewFire: () => undefined,
    probeConnection: async () => ({
      quality: Quality.Good,
      toast: false,
      alarm: false,
      restored: false,
      nextPollMs: 10_000n,
    }),
    connectionRetry: () => {},
    connectionReset: () => {},
    connectionHealthy: () => true,
    ...createMockGates(),
    ...createMockScene(bus),
  };
}
