import { Linking } from 'react-native';
import InAppBrowser from 'react-native-inappbrowser-reborn';
import { core } from '../core';
import { onCoreEvents } from '../core/events';
import { StartKind } from '../core/native';

let started = false;

/**
 * Opens the pages the core asks for (sign-in) and sends every `decentraland://` link back to it.
 * Called once at startup; the browser returns through the deep link, not through this call.
 */
export function startLinkHandling() {
  if (started) {
    return;
  }
  started = true;
  onCoreEvents({
    openUrl: ({ url }) => {
      InAppBrowser.isAvailable()
        .then(available =>
          available ? InAppBrowser.open(url) : Linking.openURL(url),
        )
        .catch(() => Linking.openURL(url));
    },
  });
  Linking.addEventListener('url', ({ url }) => routeLink(url, StartKind.Warm));
  Linking.getInitialURL().then(url => url && routeLink(url, StartKind.Cold));
}

function routeLink(url: string, start: StartKind) {
  InAppBrowser.close();
  core().handleDeepLink(url, start);
}
