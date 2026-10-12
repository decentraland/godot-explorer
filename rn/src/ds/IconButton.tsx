import { Pressable, StyleSheet } from 'react-native';
import { Icon, IconName } from './Icon';
import { colors, radii } from './tokens';

export function IconButton({
  icon,
  label,
  onPress,
  size = 30,
  active = false,
}: {
  icon: IconName;
  label: string;
  onPress: () => void;
  size?: number;
  active?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      onPress={onPress}
      hitSlop={8}
      style={({ pressed }) => [
        styles.button,
        { width: size, height: size },
        active && styles.active,
        pressed && styles.pressed,
      ]}
    >
      <Icon name={icon} size={size * 0.55} />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: radii.button,
    backgroundColor: colors.purple550,
  },
  active: { backgroundColor: colors.purple450 },
  pressed: { opacity: 0.8 },
});
