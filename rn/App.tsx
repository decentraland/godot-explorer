/**
 * RN entry flow, then the tab shell. The core starts the engine when a screen needs it: the avatar
 * preview first, then the world on Jump In.
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { StatusBar, StyleSheet, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { onLinkTarget } from './src/app/links';
import type { LinkTarget } from './src/app/links';
import { NavProvider, useNav } from './src/app/NavProvider';
import { gallery, usePreviewRoute } from './src/app/preview';
import { Shell } from './src/app/Shell';
import { useAnalyticsLifecycle } from './src/app/useAnalyticsLifecycle';
import { startApp } from './src/app/startup';
import { useLocaleSync } from './src/app/useLocaleSync';
import { useSessionTracking } from './src/app/useSessionTracking';
import { CoreModals } from './src/modals/CoreModals';
import { ModalProvider } from './src/modals/ModalProvider';
import { usePushRegistration } from './src/notifications/usePushRegistration';
import { useAppWiring } from './src/app/wiring';
import { core } from './src/core';
import { useCoreState } from './src/core/useCoreState';
import { useJumpIn } from './src/discover/sheetHooks';
import { colors, ToastProvider } from './src/ds';
import { runCoreSelfTest } from './src/core/selfTest';
import { EntryFlow } from './src/flow/EntryFlow';

function AppBody() {
  const [entered, setEntered] = useState(false);
  const { push, reset, route } = useNav();
  const onDone = useCallback(() => setEntered(true), []);

  const locale = useLocaleSync();
  const jumpIn = useJumpIn();
  useAnalyticsLifecycle(entered);
  useEffect(() => {
    core().firstScreenDrawn();
  }, []);

  // Signing out or deleting the account sends the player back to the entry flow.
  const coreTag = useCoreState().tag;
  const signedOut = coreTag === 'SignedOut';
  usePushRegistration(
    coreTag !== 'SignedOut' && coreTag !== 'Booting' && coreTag !== 'SigningIn',
  );
  useEffect(() => {
    if (signedOut && entered) {
      setEntered(false);
      reset({ name: 'home' });
    }
  }, [signedOut, entered, reset]);

  // A deep link navigates once the player is past the entry flow; one that arrives earlier waits.
  const waitingLink = useRef<LinkTarget | undefined>(undefined);
  const openLink = useCallback(
    (target: LinkTarget) => {
      if (target.kind === 'world') {
        jumpIn(target.place);
      } else if (target.kind === 'place') {
        push({ name: 'placeDetail', params: { placeId: target.placeId } });
      } else if (target.kind === 'event') {
        push({ name: 'eventDetail', params: { eventId: target.eventId } });
      } else {
        reset({ name: 'home' });
      }
    },
    [jumpIn, push, reset],
  );
  useEffect(
    () =>
      onLinkTarget(target => {
        if (entered) {
          openLink(target);
        } else {
          waitingLink.current = target;
        }
      }),
    [entered, openLink],
  );
  useEffect(() => {
    if (entered && waitingLink.current) {
      const target = waitingLink.current;
      waitingLink.current = undefined;
      openLink(target);
    }
  }, [entered, openLink]);

  const leave = useCallback(() => {
    try {
      core().exitWorld();
    } catch {
      // Not in a world flow (a refused entry already left it).
    } finally {
      reset({ name: 'home' });
    }
  }, [reset]);
  useAppWiring(leave);

  // The world draws Godot underneath, so its background must stay transparent.
  const inWorld = route.name === 'world';
  return (
    <View key={locale} style={inWorld ? styles.transparent : styles.root}>
      <CoreModals ready={entered} />
      {entered ? (
        <Shell onJumpIn={jumpIn} onLeave={leave} />
      ) : (
        <EntryFlow
          onDone={onDone}
          onJumpIn={place => {
            onDone();
            return jumpIn(place);
          }}
        />
      )}
    </View>
  );
}

function App() {
  const preview = usePreviewRoute();
  useSessionTracking();
  useEffect(() => {
    startApp();
    if (__DEV__) {
      runCoreSelfTest();
    }
  }, []);
  const Host = gallery?.GalleryHost;

  return (
    <SafeAreaProvider>
      <View style={styles.transparent}>
        <StatusBar hidden />
        {!preview.ready ? null : Host && preview.route ? (
          <Host {...preview.route} />
        ) : (
          <ToastProvider>
            <ModalProvider>
              <NavProvider>
                <AppBody />
              </NavProvider>
            </ModalProvider>
          </ToastProvider>
        )}
      </View>
    </SafeAreaProvider>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.purple550 },
  transparent: { flex: 1, backgroundColor: 'transparent' },
});

export default App;
