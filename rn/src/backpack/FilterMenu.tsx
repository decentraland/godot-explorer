import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon } from '../ds';
import { ItemFilter } from './selection';

const OPTIONS: { key: ItemFilter; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'collectibles', label: 'Collectibles' },
];

/** Dropdown under the filter button (Figma "Menu"). */
export function FilterMenu({
  value,
  onChange,
}: {
  value: ItemFilter;
  onChange: (next: ItemFilter) => void;
}) {
  return (
    <View style={styles.menu}>
      {OPTIONS.map(option => {
        const selected = option.key === value;
        return (
          <Pressable
            key={option.key}
            accessibilityRole="menuitem"
            accessibilityState={{ selected }}
            onPress={() => onChange(option.key)}
            style={[styles.row, selected && styles.rowOn]}
          >
            <AppText variant="body" style={styles.label}>
              {option.label}
            </AppText>
            {selected && (
              <Icon name="check" size={18} color={colors.lavender} />
            )}
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  menu: {
    width: 165,
    padding: 8,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: 'rgba(232,185,255,0.45)',
    backgroundColor: colors.purple550,
  },
  row: {
    height: 42,
    paddingHorizontal: 8,
    borderRadius: 6,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  rowOn: { backgroundColor: 'rgba(0,0,0,0.35)' },
  label: { fontSize: 16, color: colors.lavender },
});
