/**
 * @format
 */

import { AppRegistry } from 'react-native';
import App from './App';
import { name as appName } from './app.json';
import { initSentry } from './src/sentry';

// First, so a crash while the rest of the app starts is already caught.
initSentry();

AppRegistry.registerComponent(appName, () => App);
// Expo Go opens the component registered as "main" (UI work without the native app or Godot).
AppRegistry.registerComponent('main', () => App);
