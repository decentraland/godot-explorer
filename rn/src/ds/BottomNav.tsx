import { Pressable, StyleSheet, View } from 'react-native';
import { colors } from './tokens';
import { Icon } from './Icon';
import { u } from './scale';

export type NavKey = 'explore' | 'backpack' | 'settings' | 'profile';

const BOX = 41;
const COMPASS = 35;

function NavIcon({ kind, active }: { kind: NavKey; active: boolean }) {
  switch (kind) {
    case 'explore':
      return (
        <View style={styles.rotated}>
          <Icon
            name={active ? 'compassActive' : 'compassOutline'}
            size={COMPASS}
          />
        </View>
      );
    case 'backpack':
      return <Icon name={active ? 'backpack' : 'backpackOutline'} size={24} />;
    case 'settings':
      return <Icon name={active ? 'settingsActive' : 'settings'} size={25} />;
    case 'profile':
      return (
        <View style={styles.profile}>
          <View style={StyleSheet.absoluteFill}>
            <Icon name="profileFill" size={36} />
          </View>
          <View style={StyleSheet.absoluteFill}>
            <Icon name="profileRing" size={36} />
          </View>
          <Icon
            name="user"
            size={18}
            color={colors.purple450}
            strokeWidth={2.2}
          />
        </View>
      );
  }
}

const ORDER: NavKey[] = ['explore', 'backpack', 'settings', 'profile'];

/** Four-icon bottom bar (Figma "NavBar"): the active destination is filled with the purple gradient. */
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
    <View style={[styles.bar, { paddingBottom: u(20) + bottomInset }]}>
      {ORDER.map(key => (
        <Pressable
          key={key}
          accessibilityRole="tab"
          accessibilityLabel={key}
          accessibilityState={{ selected: key === active }}
          hitSlop={u(8)}
          onPress={() => onChange(key)}
          style={({ pressed }) => [styles.item, pressed && styles.pressed]}
        >
          <NavIcon kind={key} active={key === active} />
        </Pressable>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  bar: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(28),
    paddingTop: u(16),
    paddingHorizontal: u(32),
    backgroundColor: colors.shadow,
  },
  item: {
    width: u(BOX),
    height: u(BOX),
    alignItems: 'center',
    justifyContent: 'center',
  },
  rotated: { transform: [{ rotate: '-45deg' }] },
  profile: {
    width: u(36),
    height: u(36),
    alignItems: 'center',
    justifyContent: 'center',
  },
  pressed: { opacity: 0.7 },
});
