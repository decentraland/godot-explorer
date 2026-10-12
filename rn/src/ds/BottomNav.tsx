import { Pressable, StyleSheet, View } from 'react-native';
import { Icon } from './Icon';
import { colors, layout } from './tokens';

export type NavKey = 'explore' | 'backpack' | 'settings' | 'profile';

const entries: {
  key: NavKey;
  icon: 'compass' | 'backpack' | 'settings' | 'profile';
}[] = [
  { key: 'explore', icon: 'compass' },
  { key: 'backpack', icon: 'backpack' },
  { key: 'settings', icon: 'settings' },
  { key: 'profile', icon: 'profile' },
];

export function BottomNav({
  active,
  bottomInset = 0,
  onChange,
}: {
  active: NavKey;
  bottomInset?: number;
  onChange: (key: NavKey) => void;
}) {
  return (
    <View
      style={[
        styles.bar,
        { height: layout.navHeight + bottomInset, paddingBottom: bottomInset },
      ]}
    >
      {entries.map(entry => (
        <Pressable
          key={entry.key}
          accessibilityRole="tab"
          accessibilityLabel={entry.key}
          accessibilityState={{ selected: entry.key === active }}
          onPress={() => onChange(entry.key)}
          style={({ pressed }) => [styles.item, pressed && styles.pressed]}
        >
          <Icon name={entry.icon} size={entry.key === 'backpack' ? 24 : 26} />
          <View style={[styles.dot, entry.key === active && styles.dotOn]} />
        </Pressable>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  bar: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-around',
    backgroundColor: colors.shadow,
    paddingHorizontal: 24,
  },
  item: {
    width: 56,
    height: 56,
    alignItems: 'center',
    justifyContent: 'center',
  },
  dot: {
    width: 4,
    height: 4,
    borderRadius: 2,
    marginTop: 4,
    backgroundColor: 'transparent',
  },
  dotOn: { backgroundColor: colors.ruby },
  pressed: { opacity: 0.7 },
});
