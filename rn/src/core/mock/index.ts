import type { CoreApi } from '../api';
import { createMockControl } from './control';
import { createMockSession } from './session';
import { createMockSettings } from './settings';
import { createMockSocial } from './social';
import { createMockProfile } from './profile';
import { createMockPlaces } from './places';
import { createMockGates } from './gates';
import { createMockScene } from './scene';

/** Implements the same interface as the native binding, for builds without the engine library. */
export function createMockCore(): CoreApi {
  return {
    ...createMockSession(),
    ...createMockControl(),
    ...createMockSettings(),
    ...createMockSocial(),
    ...createMockProfile(),
    ...createMockPlaces(),
    ...createMockGates(),
    ...createMockScene(),
  };
}
