/**
 * RN entry flow, then the tab shell. Godot is booted lazily:
 * first to draw the single avatar preview, then (on Jump In) to run the world (see src/engine.ts).
 */
import { useCallback, useState } from 'react';
import { StatusBar, StyleSheet, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { NavProvider, useNav } from './src/app/NavProvider';
import { gallery, usePreviewRoute } from './src/app/preview';
import { Shell } from './src/app/Shell';
import { useAppWiring } from './src/app/wiring';
import { useJumpIn } from './src/discover/sheetHooks';
import { colors, ToastProvider } from './src/ds';
import { send } from './src/engine';
import { EntryFlow } from './src/flow/EntryFlow';

function AppBody() {
  const [entered, setEntered] = useState(false);
  const { reset, route } = useNav();
  const onDone = useCallback(() => setEntered(true), []);

  const jumpIn = useJumpIn();

  const leave = useCallback(() => {
    send({ type: 'exit_world' });
    reset({ name: 'home' });
  }, [reset]);
  useAppWiring(leave);

  // The world draws Godot underneath, so its background must stay transparent.
  const inWorld = route.name === 'world';
  return (
    <View style={inWorld ? styles.transparent : styles.root}>
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
