import { Pressable, StyleSheet, View } from 'react-native';
import { colors } from '../ds';

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
    width: 34,
    height: 34,
    borderRadius: 17,
    borderWidth: 2,
    borderColor: colors.ruby,
    alignItems: 'center',
    justifyContent: 'center',
  },
  dot: {
    width: 24,
    height: 24,
    borderRadius: 12,
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.5)',
  },
});
