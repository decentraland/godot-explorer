import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { colors } from './tokens';

export type TabItem<K extends string> = { key: K; label: string };

/** Uppercase tab strip with a ruby underline under the active tab. */
export function Tabs<K extends string>({
  items,
  active,
  onChange,
}: {
  items: TabItem<K>[];
  active: K;
  onChange: (key: K) => void;
}) {
  return (
    <View style={styles.row}>
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
            <AppText variant="tab" style={selected && styles.selectedText}>
              {item.label}
            </AppText>
            <View style={[styles.underline, selected && styles.underlineOn]} />
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', justifyContent: 'space-between' },
  tab: { paddingVertical: 4, alignItems: 'center' },
  selectedText: { color: colors.snow },
  underline: {
    height: 2,
    alignSelf: 'stretch',
    marginTop: 4,
    backgroundColor: 'transparent',
  },
  underlineOn: { backgroundColor: colors.ruby },
});
