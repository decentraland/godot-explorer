const fs = require('fs');
const path = require('path');

// The core binding links libdclgodot.so, which only the Godot build has (scripts/sync-godot-android.sh
// fills android/app/godot); the no-engine builds run on the mock core. iOS joins with the iOS host.
const withGodot =
  fs.existsSync(path.join(__dirname, 'android/app/godot')) && !process.env.DCL_NO_GODOT;

// `expo` is here for `expo start` / Expo Go (UI work without the native app); the native apps do
// not integrate Expo modules, so keep autolinking from adding its pods and Gradle project.
module.exports = {
  dependencies: {
    expo: { platforms: { ios: null, android: null } },
    'dcl-core': { platforms: withGodot ? { ios: null } : { ios: null, android: null } },
  },
};
