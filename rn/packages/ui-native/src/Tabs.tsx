import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Badge } from './Badge';
import { colors } from './tokens';
import { u } from './scale';

export type TabItem<K extends string> = {
  key: K;
  label: string;
  badge?: boolean | number;
};

/** Uppercase tab strip with a ruby underline under the active tab. */
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
  /** Fixed space between tabs (they hug their label instead of spreading). */
  gap?: number;
  /** Label font size in dp. */
  labelSize?: number;
}) {
  return (
    <View
      style={[styles.row, gap !== undefined && [styles.hug, { gap: u(gap) }]]}
    >
      {items.map(item => {
        const selected = item.key === active;
        return (
          <Pressable
            key={item.key}
            accessibilityRole="tab"
            accessibilityState={{ selected }}
            onPress={() => onChange(item.key)}
            style={styles.tab}
          >
            <View style={styles.labelRow}>
              <AppText
                variant="tab"
                style={[
                  selected && styles.selectedText,
                  labelSize !== undefined && { fontSize: u(labelSize) },
                ]}
              >
                {item.label}
              </AppText>
              {item.badge !== undefined && item.badge !== false && (
                <View style={styles.badge}>
                  <Badge
                    count={
                      typeof item.badge === 'number' ? item.badge : undefined
                    }
                  />
                </View>
              )}
            </View>
            <View style={[styles.underline, selected && styles.underlineOn]} />
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', justifyContent: 'space-between' },
  hug: { justifyContent: 'flex-start' },
  tab: { paddingVertical: u(4), alignItems: 'center' },
  labelRow: { flexDirection: 'row', alignItems: 'center' },
  badge: { marginLeft: u(4) },
  selectedText: { color: colors.snow },
  underline: {
    height: u(2),
    alignSelf: 'stretch',
    marginTop: u(4),
    backgroundColor: 'transparent',
  },
  underlineOn: { backgroundColor: colors.ruby },
});
