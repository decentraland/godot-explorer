// Expo's Metro defaults extend React Native's, so both `react-native start` and `expo start`
// (Expo Go) serve the app.
const { getDefaultConfig } = require('expo/metro-config');

module.exports = getDefaultConfig(__dirname);
