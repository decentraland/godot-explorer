// Expo's Metro defaults extend React Native's, so both `react-native start` and `expo start`
// (Expo Go) serve the app. The Sentry wrapper stamps every bundle with a debug ID, which is what
// lets Sentry match an uploaded source map to the bundle that crashed.
const { getSentryExpoConfig } = require('@sentry/react-native/metro');

module.exports = getSentryExpoConfig(__dirname);
