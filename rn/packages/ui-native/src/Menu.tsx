import {
  Pressable,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors, elevation } from './tokens';
import { u } from './scale';

export type MenuItem<K extends string> = {
  key: K;
  label: string;
  /** Leading `Icon/Menu` glyph. */
  icon?: IconName;
  selected?: boolean;
  /** Trailing check. */
  check?: boolean;
};

/** Figma `Menu`: the floating context menu (`surface/menu`, `border/menu`, `Elevation/Menu`) made of
 * `Menu/Item` rows. The caller positions it and dismisses it on an outside tap. */
export function Menu<K extends string>({
  items,
  onSelect,
  style,
}: {
  items: MenuItem<K>[];
  onSelect: (key: K) => void;
  style?: StyleProp<ViewStyle>;
}) {
  return (
    <View style={[styles.menu, style]} accessibilityRole="menu">
      {items.map(item => {
        const tint = item.selected ? colors.lavender : colors.snow;
        return (
          <Pressable
            key={item.key}
            accessibilityRole="menuitem"
            accessibilityLabel={item.label}
            accessibilityState={{ selected: !!item.selected }}
            onPress={() => onSelect(item.key)}
            style={({ pressed }) => [
              styles.row,
              (item.selected || pressed) && styles.rowSelected,
            ]}
          >
            {item.icon && (
              <View style={styles.iconSlot}>
                <Icon name={item.icon} size={18.2} color={tint} />
              </View>
            )}
            <AppText
              variant="menuItem"
              numberOfLines={1}
              style={[styles.label, { color: tint }]}
            >
              {item.label}
            </AppText>
            {item.check && <Icon name="check" size={20} color={tint} />}
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  menu: {
    width: u(160),
    paddingHorizontal: u(12),
    paddingVertical: u(8),
    gap: u(13),
    borderRadius: u(6),
    borderWidth: u(0.5),
    borderColor: colors.lavender70,
    backgroundColor: colors.menu,
    ...elevation.menu,
  },
  row: {
    height: u(32),
    paddingHorizontal: u(8),
    gap: u(5),
    borderRadius: u(6),
    flexDirection: 'row',
    alignItems: 'center',
  },
  rowSelected: { backgroundColor: colors.black40 },
  /** The node draws a 36.4 px `Icon/Base` overflowing its 32 px slot. */
  iconSlot: {
    width: u(16),
    height: u(16),
    alignItems: 'center',
    justifyContent: 'center',
  },
  label: { flex: 1 },
});
