import { TurboModuleRegistry } from 'react-native';
import type { CoreApi } from './api';
import { createMockCore } from './mock';

export type { CoreApi } from './api';

let current: CoreApi | undefined;

/** The native binding when the engine library is linked, the mock otherwise (no-engine builds,
 * jest, Expo Go). Native module load failures fall back to the mock instead of crashing. */
function load(): CoreApi {
  // Checked first: a throwing `require` is reported as a fatal error by Metro's module guard (a
  // red box in debug builds) before the catch below runs.
  if (!TurboModuleRegistry.get('DclCore')) {
    return createMockCore();
  }
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

/** Dev preview harness: in a build without the engine, the mock core already signed in, so screens that
 * drive the avatar preview can run. A linked engine is left alone. */
export function signedInMockCoreForPreviews() {
  if (!TurboModuleRegistry.get('DclCore')) {
    current = createMockCore({ signedIn: true });
  }
}

/** Replaces the core for a test; pass nothing to restore the default. */
export function setCoreForTests(replacement?: CoreApi) {
  current = replacement;
}
