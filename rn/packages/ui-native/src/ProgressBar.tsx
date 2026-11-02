import { StyleSheet, View } from 'react-native';
import { colors } from './tokens';
import { u } from './scale';

/** Ruby fill over a dark track; `value` is 0..1. */
export function ProgressBar({
  value,
  label,
  height = 20,
  radius = height / 2,
}: {
  value: number;
  label?: string;
  height?: number;
  /** Corner radius in dp; fully round by default. */
  radius?: number;
}) {
  const clamped = Math.min(1, Math.max(0, value));
  return (
    <View
      accessibilityRole="progressbar"
      accessibilityLabel={label}
      accessibilityValue={{ min: 0, max: 100, now: Math.round(clamped * 100) }}
      style={[styles.track, { height: u(height), borderRadius: u(radius) }]}
    >
      <View
        style={[
          styles.fill,
          { width: `${clamped * 100}%`, borderRadius: u(radius) },
        ]}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  track: {
    alignSelf: 'stretch',
    backgroundColor: colors.fieldDark,
    overflow: 'hidden',
  },
  fill: { height: '100%', backgroundColor: colors.ruby },
});
