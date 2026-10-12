/**
 * VIBE-CODED SPIKE: the app is an RN-only entry flow followed by the Backpack. Godot is booted
 * lazily and only draws the single avatar preview (see src/engine.ts).
 */
import { useCallback, useState } from 'react';
import { StatusBar, StyleSheet, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { colors } from './src/ds';
import { EntryFlow } from './src/flow/EntryFlow';
import { Home } from './src/Home';

function App() {
  const [entered, setEntered] = useState(false);
  const onDone = useCallback(() => setEntered(true), []);

  return (
    <SafeAreaProvider>
      <View style={styles.root}>
        <StatusBar hidden />
        {entered ? <Home /> : <EntryFlow onDone={onDone} />}
      </View>
    </SafeAreaProvider>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.purple550 },
});

export default App;
