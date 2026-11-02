import { Pressable, StyleSheet, View } from 'react-native';
import { RadialFill } from './Gradient';
import { Icon, IconName } from './Icon';
import { NotificationBadge } from './NotificationBadge';
import { colors, elevation } from './tokens';
import { u } from './scale';

export type NavKey = 'explore' | 'backpack' | 'settings' | 'profile';

const ICONS: Record<NavKey, { idle: IconName; active: IconName }> = {
  explore: { idle: 'navExplore', active: 'navExploreActive' },
  backpack: { idle: 'navBackpack', active: 'navBackpackActive' },
  settings: { idle: 'navSettings', active: 'navSettingsActive' },
  profile: { idle: 'navProfile', active: 'navProfileActive' },
};

const ORDER: NavKey[] = ['explore', 'backpack', 'settings', 'profile'];

const GLOW = [
  { offset: 0, color: colors.snow, opacity: 0.55 },
  { offset: 1, color: colors.snow, opacity: 0 },
];

/** Figma `NavBar` (`NavBar/Item` × 4): the active destination shows the gradient `Icon/NavBar` plus
 * the "Selected indicator" glow on the bar edge. Portrait is the bottom bar; `landscape` is the
 * 100px left rail. */
export function BottomNav({
  active,
  bottomInset = 0,
  badges,
  landscape = false,
  onChange,
}: {
  active: NavKey;
  bottomInset?: number;
  /** Destinations carrying a `Badge/Notification` dot. */
  badges?: Partial<Record<NavKey, boolean>>;
  landscape?: boolean;
  onChange: (key: NavKey) => void;
}) {
  const size = landscape ? 32 : 41;
  return (
    <View
      style={
        landscape
          ? styles.rail
          : [styles.bar, { paddingBottom: u(20) + bottomInset }]
      }
    >
      {ORDER.map(key => {
        const selected = key === active;
        return (
          <Pressable
            key={key}
            accessibilityRole="tab"
            accessibilityLabel={key}
            accessibilityState={{ selected }}
            hitSlop={u(8)}
            onPress={() => onChange(key)}
            style={{ width: u(size), height: u(size) }}
          >
            {selected && (
              <View
                style={[
                  styles.indicator,
                  landscape ? styles.indicatorRail : styles.indicatorBar,
                ]}
              >
                <RadialFill
                  stops={GLOW}
                  cx={landscape ? 0 : 0.5}
                  cy={landscape ? 0.5 : 0}
                  rx={landscape ? 0.7 : 0.63}
                  ry={landscape ? 0.63 : 0.7}
                />
                <View
                  style={[
                    styles.line,
                    landscape ? styles.lineRail : styles.lineBar,
                  ]}
                />
              </View>
            )}
            <Icon name={selected ? ICONS[key].active : ICONS[key].idle} size={size} />
            {badges?.[key] && <NotificationBadge style={styles.badge} />}
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  bar: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'center',
    gap: u(28),
    paddingTop: u(16),
    paddingHorizontal: u(32),
    backgroundColor: colors.navbar,
    ...elevation.up,
    shadowColor: '#000000',
  },
  rail: {
    width: u(50),
    alignItems: 'center',
    gap: u(18.5),
    paddingVertical: u(20),
    backgroundColor: colors.navbar,
  },
  indicator: { position: 'absolute', overflow: 'hidden' },
  indicatorBar: { left: -u(11), right: -u(11), top: -u(16), height: u(28) },
  indicatorRail: { top: -u(5), bottom: -u(5), left: -u(20), width: u(21) },
  line: { position: 'absolute', backgroundColor: colors.snow },
  lineBar: { top: 0, left: u(15), right: u(15), height: u(1) },
  lineRail: { left: 0, top: u(3), bottom: u(3), width: u(1) },
  badge: { position: 'absolute', top: 0, right: 0 },
});
