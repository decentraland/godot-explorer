/** Menus are portrait-locked; a scene is landscape-locked. */
export type Orientation = 'portrait' | 'landscape';

/** Locks the screen. A no-op where the native module is absent (tests, Expo Go). */
export function setOrientation(mode: Orientation): void {
  try {
    const locker = require('react-native-orientation-locker').default;
    if (mode === 'landscape') {
      locker.lockToLandscape();
    } else {
      locker.lockToPortrait();
    }
  } catch {
    // No orientation module in this build.
  }
}
