import { StyleSheet, View } from 'react-native';
import { LinearFill, RadialFill, u } from '../ds';

const SKY = [
  { offset: 0, color: '#6B5AA8' },
  { offset: 0.55, color: '#7C4C9A' },
  { offset: 1, color: '#B87A7E' },
];
const GLOW = [
  { offset: 0, color: '#FF9A6B', opacity: 0.55 },
  { offset: 1, color: '#FF9A6B', opacity: 0 },
];

// x, width, height (dp), color: skyline silhouettes standing in for a place photo.
const BUILDINGS: [number, number, number, string][] = [
  [0.3, 80, 150, '#4A2A72'],
  [0.46, 70, 190, '#3D3A8C'],
  [0.55, 150, 105, '#7A3F6E'],
  [0.74, 130, 175, '#4B2A7A'],
  [0.88, 90, 120, '#8C3A5A'],
];

/** Blurred-photo look for a place without an image: sunset sky, skyline, ground. */
export function PlaceBackdrop() {
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <LinearFill stops={SKY} />
      <RadialFill stops={GLOW} cx={0.75} cy={0.8} rx={0.5} ry={0.6} />
      {BUILDINGS.map(([x, w, h, color]) => (
        <View
          key={x}
          style={[
            styles.building,
            {
              left: `${x * 100}%`,
              width: u(w),
              height: u(h),
              backgroundColor: color,
            },
          ]}
        />
      ))}
      <View style={styles.ground} />
    </View>
  );
}

const styles = StyleSheet.create({
  building: {
    position: 'absolute',
    bottom: '22%',
    borderTopLeftRadius: u(6),
    borderTopRightRadius: u(6),
    opacity: 0.85,
  },
  ground: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: '24%',
    backgroundColor: '#8A5A78',
    opacity: 0.7,
  },
});
