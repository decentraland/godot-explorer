const fs = require('fs');
const path = require('path');

// The core binding links libdclgodot, which only the Godot build has (scripts/sync-godot-android.sh
// fills android/app/godot, scripts/sync-godot-ios.sh ios/Godot); the no-engine builds run on the
// mock core.
const synced = dir =>
  fs.existsSync(path.join(__dirname, dir)) && !process.env.DCL_NO_GODOT;
const withGodotAndroid = synced('android/app/godot');
const withGodotIos = synced('ios/Godot/godot.xcconfig');

// `expo` is here for `expo start` / Expo Go (UI work without the native app); the native apps do
// not integrate Expo modules, so keep autolinking from adding its pods and Gradle project.
module.exports = {
  dependencies: {
    expo: { platforms: { ios: null, android: null } },
    'dcl-core': {
      platforms: {
        ...(withGodotIos ? {} : { ios: null }),
        ...(withGodotAndroid ? {} : { android: null }),
      },
    },
  },
};
