import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { NotificationBadge } from './NotificationBadge';
import { color, radius, space } from './tokens';
import { u } from './scale';

/** Navigation row of a settings-style list: label and a right chevron. Selected: Surface/selected
 * with the bold label. `badge` shows the notification dot at the label's top right. Large fills the width (portrait), Small is the landscape side panel's.
 * Figma Menu/NavItem: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=917-1329
 */
export function MenuNavItem({
  label,
  selected = false,
  size = 'Large',
  badge = false,
  onPress,
}: {
  label: string;
  selected?: boolean;
  size?: 'Large' | 'Small';
  badge?: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected }}
      onPress={onPress}
      style={[
        styles.item,
        size === 'Large' ? styles.large : styles.small,
        selected && styles.selected,
      ]}
    >
      <View style={styles.label}>
        <AppText variant={selected ? 'Menu/NavItem Selected' : 'Menu/NavItem'}>
          {label}
        </AppText>
        {badge && <NotificationBadge size={8} style={styles.badge} />}
      </View>
      <Icon name="chevronRight" size={16} color={color.icon.default} />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  item: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: space.xs,
    padding: space.sm,
    borderRadius: radius.sm,
  },
  large: { height: u(36), alignSelf: 'stretch' },
  small: { width: u(188) },
  selected: { backgroundColor: color.surface.selected },
  label: { flexDirection: 'row', alignItems: 'flex-start' },
  badge: { marginLeft: -u(1.5) },
});
