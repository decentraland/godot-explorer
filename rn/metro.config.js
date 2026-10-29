// Expo's Metro defaults extend React Native's, so both `react-native start` and `expo start`
// (Expo Go) serve the app. The Sentry wrapper stamps every bundle with a debug ID, which is what
// lets Sentry match an uploaded source map to the bundle that crashed.
const { getSentryExpoConfig } = require('@sentry/react-native/metro');
const {
  withStorybook,
} = require('@storybook/react-native/metro/withStorybook');

// Storybook is in the bundle only when EXPO_PUBLIC_STORYBOOK_ENABLED=true (the visual snapshots).
module.exports = withStorybook(getSentryExpoConfig(__dirname), {
  enabled: process.env.EXPO_PUBLIC_STORYBOOK_ENABLED === 'true',
});
