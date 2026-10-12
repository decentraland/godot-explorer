/**
 * VIBE-CODED SPIKE: an RN-only entry flow, then Discover / Backpack. Godot is booted lazily:
 * first to draw the single avatar preview, then (on Jump In) to run the world (see src/engine.ts).
 */
import { useCallback, useState } from 'react';
import { StatusBar, StyleSheet, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { Place } from './src/discover/api';
import { recordPlayed } from './src/discover/history';
import { colors } from './src/ds';
import { ensureEngine, send } from './src/engine';
import { EntryFlow } from './src/flow/EntryFlow';
import { Home } from './src/Home';
import { WorldScreen } from './src/world/WorldScreen';

function App() {
  const [entered, setEntered] = useState(false);
  const [world, setWorld] = useState<Place | null>(null);
  const onDone = useCallback(() => setEntered(true), []);

  const jumpIn = useCallback((place: Place) => {
    recordPlayed(place);
    ensureEngine();
    send({ type: 'enter_world', x: place.x, y: place.y, realm: place.realm });
    setWorld(place);
  }, []);

  const leave = useCallback(() => {
    send({ type: 'exit_world' });
    setWorld(null);
  }, []);

  return (
    <SafeAreaProvider>
      <View style={world ? styles.transparent : styles.root}>
        <StatusBar hidden />
        {world ? (
          <WorldScreen place={world} onLeave={leave} />
        ) : entered ? (
          <Home onJumpIn={jumpIn} />
        ) : (
          <EntryFlow onDone={onDone} />
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
