import { Pressable, StyleSheet, View } from 'react-native';
import { colors } from './tokens';
import { u } from './scale';

const DISABLED_TRACK = '#39005A';
const DISABLED_THUMB = '#402D4F';

/** Settings toggle: 70x32dp track, ruby when on. */
export function Switch({
  value,
  onChange,
  label,
  disabled = false,
}: {
  value: boolean;
  onChange: (value: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="switch"
      accessibilityLabel={label}
      accessibilityState={{ checked: value, disabled }}
      disabled={disabled}
      hitSlop={8}
      onPress={() => onChange(!value)}
      style={[
        styles.track,
        value ? styles.on : styles.off,
        disabled && (value ? styles.disabled : styles.offDisabled),
      ]}
    >
      <View
        style={[
          styles.thumb,
          value && styles.thumbOn,
          disabled && !value && styles.thumbDisabled,
        ]}
      />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  track: {
    width: u(70),
    height: u(32),
    borderRadius: u(16),
    padding: u(3),
    justifyContent: 'center',
  },
  on: { backgroundColor: colors.ruby },
  off: { backgroundColor: colors.fieldDark },
  disabled: { opacity: 0.35 },
  offDisabled: { backgroundColor: DISABLED_TRACK },
  thumbDisabled: { backgroundColor: DISABLED_THUMB },
  thumb: {
    width: u(26),
    height: u(26),
    borderRadius: u(13),
    backgroundColor: colors.snow,
  },
  thumbOn: { alignSelf: 'flex-end' },
});
