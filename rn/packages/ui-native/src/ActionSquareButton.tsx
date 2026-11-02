import { Pressable, StyleSheet } from 'react-native';
import { Icon, IconName } from './Icon';
import { colors } from './tokens';
import { u } from './scale';

/** 47dp violet square icon button used on sheets (share, calendar, like, favorite). */
export function ActionSquareButton({
  icon,
  label,
  onPress,
  active = false,
  disabled = false,
}: {
  icon: IconName;
  label: string;
  onPress: () => void;
  active?: boolean;
  disabled?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected: active, disabled }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        disabled && styles.disabled,
        pressed && styles.pressed,
      ]}
    >
      <Icon name={icon} size={22} color={active ? colors.ruby : colors.snow} />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    width: u(47),
    height: u(47),
    borderRadius: u(8),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.violetSquare,
  },
  disabled: { opacity: 0.35 },
  pressed: { opacity: 0.8 },
});
