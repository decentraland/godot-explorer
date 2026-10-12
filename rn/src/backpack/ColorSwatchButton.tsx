import { Pressable, StyleSheet, View } from 'react-native';
import { colors } from '../ds';
import { u } from '../ds/scale';

/** Round swatch showing the current color; opens the color panel. */
export function ColorSwatchButton({
  color,
  onPress,
}: {
  color: string;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel="Change color"
      onPress={onPress}
      style={styles.ring}
    >
      <View style={[styles.dot, { backgroundColor: color }]} />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  ring: {
    width: u(34),
    height: u(34),
    borderRadius: u(17),
    borderWidth: 2,
    borderColor: colors.ruby,
    alignItems: 'center',
    justifyContent: 'center',
  },
  dot: {
    width: u(24),
    height: u(24),
    borderRadius: u(12),
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.5)',
  },
});
