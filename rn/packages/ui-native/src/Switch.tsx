import { Pressable, StyleSheet, View } from 'react-native';
import { colors, radii } from './tokens';
import { u } from './scale';

const SIZES = {
  lg: { width: 70, height: 32, pad: 2, knobW: 35 },
  sm: { width: 55, height: 28, pad: 1.75, knobW: 30 },
} as const;

/** Figma `Control/Switch`: Ruby track when on, Black 40% when off; `lg` 140×64, `sm` 110×56. */
export function Switch({
  value,
  onChange,
  label,
  disabled = false,
  size = 'lg',
}: {
  value: boolean;
  onChange: (value: boolean) => void;
  label: string;
  disabled?: boolean;
  size?: 'lg' | 'sm';
}) {
  const s = SIZES[size];
  return (
    <Pressable
      accessibilityRole="switch"
      accessibilityLabel={label}
      accessibilityState={{ checked: value, disabled }}
      disabled={disabled}
      hitSlop={u(8)}
      onPress={() => onChange(!value)}
      style={[
        styles.track,
        { width: u(s.width), height: u(s.height), padding: u(s.pad) },
        value ? styles.knobEnd : styles.knobStart,
        value ? styles.on : styles.off,
        disabled && (value ? styles.onDisabled : styles.offDisabled),
      ]}
    >
      <View
        style={[
          styles.knob,
          { width: u(s.knobW) },
          disabled && (value ? styles.knobOnDisabled : styles.knobOffDisabled),
        ]}
      />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  track: { borderRadius: radii.pill, justifyContent: 'center' },
  knobStart: { alignItems: 'flex-start' },
  knobEnd: { alignItems: 'flex-end' },
  on: { backgroundColor: colors.ruby },
  off: { backgroundColor: colors.black40 },
  onDisabled: { opacity: 0.5 },
  offDisabled: { backgroundColor: colors.black20 },
  knob: {
    alignSelf: 'auto',
    height: '100%',
    borderRadius: radii.pill,
    backgroundColor: colors.snow,
  },
  knobOnDisabled: { backgroundColor: colors.gray, opacity: 0.9 },
  knobOffDisabled: { backgroundColor: colors.paleBlack },
});
