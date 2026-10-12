// `expo` is here for `expo start` / Expo Go (UI work without the native app); the native apps do
// not integrate Expo modules, so keep autolinking from adding its pods and Gradle project.
module.exports = {
  dependencies: {
    expo: { platforms: { ios: null, android: null } },
  },
};
