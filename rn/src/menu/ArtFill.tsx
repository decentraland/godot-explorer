import { StyleSheet, View } from 'react-native';
import { LinearFill, u } from '../ds';

const PALETTES = [
  ['#2A6FB5', '#C7742E'],
  ['#7A2BB8', '#E0568A'],
  ['#1F8A70', '#E6B94A'],
  ['#C65A2E', '#2C3E94'],
];

const hash = (seed: string) =>
  [...seed].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 1_000_003, 7);

/** Stand-in artwork for a scene or screenshot without an image: a two-tone gradient and a skyline. */
export function ArtFill({ seed = '' }: { seed?: string }) {
  const [a, b] = PALETTES[hash(seed) % PALETTES.length];
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <LinearFill
        angle={150}
        stops={[
          { offset: 0, color: a },
          { offset: 1, color: b },
        ]}
      />
      <View style={[styles.block, styles.b1]} />
      <View style={[styles.block, styles.b2]} />
      <View style={[styles.block, styles.b3]} />
    </View>
  );
}

const styles = StyleSheet.create({
  block: {
    position: 'absolute',
    bottom: 0,
    backgroundColor: 'rgba(20,6,40,0.42)',
  },
  b1: { left: '8%', width: '22%', height: '38%', borderTopLeftRadius: u(2) },
  b2: { left: '36%', width: '28%', height: '55%' },
  b3: { left: '70%', width: '20%', height: '30%', borderTopRightRadius: u(2) },
});
