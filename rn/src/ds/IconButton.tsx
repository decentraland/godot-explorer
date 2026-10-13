import { Pressable, StyleSheet, View } from 'react-native';
import { Badge } from './Badge';
import { Icon, IconName } from './Icon';
import { colors, radii } from './tokens';
import { u } from './scale';

export function IconButton({
  icon,
  label,
  onPress,
  size = 30,
  active = false,
  badge,
}: {
  icon: IconName;
  label: string;
  onPress: () => void;
  size?: number;
  active?: boolean;
  /** `true` draws a dot, a number draws a count. */
  badge?: boolean | number;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      onPress={onPress}
      hitSlop={8}
      style={({ pressed }) => [
        styles.button,
        { width: u(size), height: u(size) },
        active && styles.active,
        pressed && styles.pressed,
      ]}
    >
      <Icon name={icon} size={size * 0.55} />
      {badge !== undefined && badge !== false && (
        <View style={styles.badge}>
          <Badge count={typeof badge === 'number' ? badge : undefined} />
        </View>
      )}
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
  badge: { position: 'absolute', top: -u(4), right: -u(4) },
});
