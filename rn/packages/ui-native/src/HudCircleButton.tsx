import { Pressable, StyleSheet, View } from 'react-native';
import { Badge } from './Badge';
import { Icon, IconName } from './Icon';
import { colors } from './tokens';
import { u } from './scale';

/** Translucent round in-world HUD button (chat, emotes, menu). */
export function HudCircleButton({
  icon,
  label,
  onPress,
  badge,
  active = false,
  size = 40,
}: {
  icon: IconName;
  label: string;
  onPress: () => void;
  badge?: boolean | number;
  active?: boolean;
  size?: number;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected: active }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        { width: u(size), height: u(size), borderRadius: u(size / 2) },
        active && styles.active,
        pressed && styles.pressed,
      ]}
    >
      <Icon name={icon} size={size * 0.5} color={colors.iconHud} />
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
    backgroundColor: 'rgba(36,3,59,0.6)',
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.3)',
  },
  active: { backgroundColor: colors.purple350 },
  pressed: { opacity: 0.8 },
  badge: { position: 'absolute', top: -u(2), right: -u(2) },
});
