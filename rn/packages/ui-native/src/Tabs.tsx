import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Badge } from './Badge';
import { Icon, IconName } from './Icon';
import { NotificationBadge } from './NotificationBadge';
import { colors } from './tokens';
import { u } from './scale';

export type TabItem<K extends string> = {
  key: K;
  label: string;
  /** `true` is Figma's `Notification` dot at the top right; a number is an inline count pill. */
  badge?: boolean | number;
  /** An `Icon/Settings` glyph before the label. */
  icon?: IconName;
};

/** Figma `Tab/Underline`: uppercase label, Snow + 6px Ruby underline when selected, Gray otherwise.
 * Each tab hugs its label. */
export function Tabs<K extends string>({
  items,
  active,
  onChange,
  gap,
  labelSize,
}: {
  items: TabItem<K>[];
  active: K;
  onChange: (key: K) => void;
  /** Fixed space between tabs instead of spreading them across the row. */
  gap?: number;
  /** Label font size in dp (default 14). */
  labelSize?: number;
}) {
  return (
    <View
      style={[styles.row, gap !== undefined && [styles.hug, { gap: u(gap) }]]}
    >
      {items.map(item => {
        const selected = item.key === active;
        const color = selected ? colors.snow : colors.gray;
        return (
          <Pressable
            key={item.key}
            accessibilityRole="tab"
            accessibilityLabel={item.label}
            accessibilityState={{ selected }}
            onPress={() => onChange(item.key)}
            style={styles.tab}
          >
            {item.icon && <Icon name={item.icon} size={16} color={color} />}
            <AppText
              variant="tab"
              style={[
                { color },
                // An explicit line keeps Android's font padding out of the baseline.
                { lineHeight: u((labelSize ?? 14) * 1.2) },
                labelSize !== undefined && { fontSize: u(labelSize) },
              ]}
            >
              {item.label}
            </AppText>
            {typeof item.badge === 'number' && (
              <View style={styles.count}>
                <Badge count={item.badge} />
              </View>
            )}
            {item.badge === true && <NotificationBadge style={styles.dot} />}
            {selected && <View style={styles.underline} />}
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', justifyContent: 'space-between' },
  hug: { justifyContent: 'flex-start' },
  tab: {
    height: u(28),
    paddingTop: u(4),
    paddingBottom: u(8),
    paddingHorizontal: u(2),
    gap: u(2),
    flexDirection: 'row',
    alignItems: 'center',
  },
  count: { marginLeft: u(2) },
  dot: {
    position: 'absolute',
    top: u(2.5),
    right: -u(3),
    width: u(8),
    height: u(8),
    borderRadius: u(4),
  },
  underline: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: u(3),
    backgroundColor: colors.ruby,
  },
});
