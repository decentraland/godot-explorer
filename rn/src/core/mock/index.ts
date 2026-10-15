import type { CoreApi } from '../api';
import { State } from '../native';
import { createMockControl } from './control';
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
  return {
    ...createMockSession(bus),
    ...createMockControl(bus),
    ...createMockSettings(),
    ...createMockProfile(),
    ...createMockPlaces(),
    ...createMockGates(),
    ...createMockScene(bus),
  };
}
