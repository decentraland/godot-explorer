/**
 * RN entry flow, then the tab shell. The core starts the engine when a screen needs it: the avatar
 * preview first, then the world on Jump In.
 */
import { useCallback, useEffect, useState } from 'react';
import { StatusBar, StyleSheet, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { onLinkTarget } from './src/app/links';
import { NavProvider, useNav } from './src/app/NavProvider';
import { gallery, usePreviewRoute } from './src/app/preview';
import { Shell } from './src/app/Shell';
import { useAnalyticsLifecycle } from './src/app/useAnalyticsLifecycle';
import { startApp } from './src/app/startup';
import { useLocaleSync } from './src/app/useLocaleSync';
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
  const signedOut = useCoreState().tag === 'SignedOut';
  useEffect(() => {
    if (signedOut && entered) {
      setEntered(false);
      reset({ name: 'home' });
    }
  }, [signedOut, entered, reset]);

  // A deep link navigates once the player is past the entry flow.
  useEffect(
    () =>
      onLinkTarget(target => {
        if (!entered) {
          return;
        }
        if (target.kind === 'world') {
          jumpIn(target.place);
        } else if (target.kind === 'place') {
          push({ name: 'placeDetail', params: { placeId: target.placeId } });
        } else if (target.kind === 'event') {
          push({ name: 'eventDetail', params: { eventId: target.eventId } });
        } else {
          reset({ name: 'home' });
        }
      }),
    [entered, jumpIn, push, reset],
  );

  const leave = useCallback(() => {
    try {
      core().exitWorld();
    } finally {
      reset({ name: 'home' });
    }
  }, [reset]);
  useAppWiring(leave);

  // The world draws Godot underneath, so its background must stay transparent.
  const inWorld = route.name === 'world';
  return (
    <View key={locale} style={inWorld ? styles.transparent : styles.root}>
      {entered ? (
        <Shell onJumpIn={jumpIn} onLeave={leave} />
      ) : (
        <EntryFlow
          onDone={onDone}
          onJumpIn={place => {
            onDone();
            jumpIn(place);
          }}
        />
      )}
    </View>
  );
}

function App() {
  const preview = usePreviewRoute();
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
            <NavProvider>
              <AppBody />
            </NavProvider>
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
