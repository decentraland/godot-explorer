// Drawn where the Godot surface would be when the engine is not linked, so layouts that depend
// on the 3D view (HUD over the world, the backpack's avatar hole) can be built in a simulator.
import { StyleSheet, Text, View, ViewStyle } from 'react-native';

export function GodotPlaceholder({ label, style }: { label: string; style?: ViewStyle }) {
  return (
    <View style={[styles.root, style]} pointerEvents="none">
      <View style={styles.horizon} />
      <Text style={styles.label}>{label}</Text>
      <Text style={styles.hint}>Godot engine not linked</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    backgroundColor: '#2b2140',
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
  },
  horizon: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: '45%',
    backgroundColor: '#3d3157',
  },
  label: { color: '#e9ddff', fontSize: 18, fontWeight: '700' },
  hint: { color: '#a796c7', fontSize: 12, marginTop: 4 },
});
