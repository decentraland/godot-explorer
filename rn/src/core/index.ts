import type { CoreApi } from './api';
import { createMockCore } from './mock';

export type { CoreApi } from './api';

let current: CoreApi | undefined;

/** The native binding when the engine library is linked, the mock otherwise (no-engine builds,
 * jest, Expo Go). Native module load failures fall back to the mock instead of crashing. */
function load(): CoreApi {
  try {
    return require('dcl-core') as CoreApi;
  } catch {
    return createMockCore();
  }
}

export function core(): CoreApi {
  current ??= load();
  return current;
}

/** Replaces the core for a test; pass nothing to restore the default. */
export function setCoreForTests(replacement?: CoreApi) {
  current = replacement;
}
