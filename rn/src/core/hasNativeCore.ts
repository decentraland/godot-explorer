import { TurboModuleRegistry } from 'react-native';

/** True when the engine's TurboModule is linked into this build. */
export function hasNativeCore(): boolean {
  return TurboModuleRegistry.get('DclCore') != null;
}
