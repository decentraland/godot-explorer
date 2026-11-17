import { Pressable, StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { color, radius, space } from './tokens';
import { u } from './scale';

/** Navigation row of a settings-style list: label and a right chevron. Selected: Surface/selected
 * with the bold label. Large fills the width (portrait), Small is the landscape side panel's.
 * Figma Menu/NavItem: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=917-1329
 */
export function MenuNavItem({
  label,
  selected = false,
  size = 'Large',
  onPress,
}: {
  label: string;
  selected?: boolean;
  size?: 'Large' | 'Small';
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
      <AppText variant={selected ? 'Menu/NavItem Selected' : 'Menu/NavItem'}>
        {label}
      </AppText>
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
});
