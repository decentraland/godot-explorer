import {
  Pressable,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors, elevation, fonts } from './tokens';
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
            {item.icon && <Icon name={item.icon} size={16} color={tint} />}
            <AppText numberOfLines={1} style={[styles.label, { color: tint }]}>
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
    width: u(159.5),
    paddingHorizontal: u(12),
    paddingVertical: u(8),
    gap: u(13),
    borderRadius: u(6),
    borderWidth: 1,
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
  label: { flex: 1, fontFamily: fonts.medium, fontSize: u(14) },
});
