import { Pressable, StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors, radii } from './tokens';
import { u } from './scale';

/** Transparent button with a 1dp border (SIGN OUT, CLEAN CACHE, REMIND ME). */
export function OutlinedButton({
  label,
  onPress,
  disabled = false,
  tone = 'default',
  icon,
  box = false,
}: {
  label: string;
  onPress: () => void;
  disabled?: boolean;
  tone?: 'default' | 'accent';
  /** Leading icon, drawn in the label colour. */
  icon?: IconName;
  /** Filled rectangle with a small radius instead of the transparent pill. */
  box?: boolean;
}) {
  const color = tone === 'accent' ? colors.ruby : colors.snow;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ disabled }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        box && styles.box,
        { borderColor: color },
        disabled && styles.disabled,
        pressed && styles.pressed,
      ]}
    >
      {icon && <Icon name={icon} size={20} color={color} />}
      <AppText variant="button" style={{ color }}>
        {label}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    minHeight: u(40),
    paddingHorizontal: u(20),
    borderRadius: radii.pill,
    borderWidth: 1,
    flexDirection: 'row',
    gap: u(8),
    alignItems: 'center',
    justifyContent: 'center',
  },
  box: {
    minHeight: u(48),
    borderRadius: u(6),
    backgroundColor: colors.fieldDark,
  },
  disabled: { opacity: 0.35 },
  pressed: { opacity: 0.8 },
});
