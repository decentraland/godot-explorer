import { ReactNode } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { Badge, colors, Icon, IconName, LinearFill, u } from '../ds';
import { t } from '../i18n';
import { Avatar } from './PersonRow';
import type { MenuSection } from './drawerState';

export const RAIL_W = u(50);
export const PANEL_W = u(312);
export const DRAWER_H = u(320);
export const DRAWER_LEFT = u(54);
export const DRAWER_TOP = u(16);

const RAIL: {
  key: MenuSection;
  label: string;
  off: IconName;
  on: IconName;
  filled?: boolean;
}[] = [
  {
    key: 'discover',
    label: 'MENU_DISCOVER',
    off: 'compass',
    on: 'compassFilled',
    filled: true,
  },
  {
    key: 'social',
    label: 'MENU_SOCIAL',
    off: 'peopleHeart',
    on: 'peopleHeart',
    filled: true,
  },
  {
    key: 'notifications',
    label: 'MENU_NOTIFICATIONS',
    off: 'bell',
    on: 'bell',
    filled: true,
  },
  {
    key: 'backpack',
    label: 'MENU_BACKPACK',
    off: 'backpackOutline',
    on: 'backpack',
  },
  {
    key: 'settings',
    label: 'MENU_SETTINGS',
    off: 'settings',
    on: 'settings',
  },
];

const RAIL_PAD = u(20);
const ITEM_H = u(40);

/** The rail, and one of its entries, on screen: entries are spread evenly with the profile last. */
export function railRect(section?: MenuSection) {
  const rail = {
    left: DRAWER_LEFT,
    top: DRAWER_TOP,
    width: RAIL_W,
    height: DRAWER_H,
  };
  if (!section) {
    return rail;
  }
  const count = RAIL.length + 1;
  const gap = (DRAWER_H - 2 * RAIL_PAD - count * ITEM_H) / (count - 1);
  const found = RAIL.findIndex(item => item.key === section);
  const index = found < 0 ? RAIL.length : found;
  return {
    ...rail,
    top: DRAWER_TOP + RAIL_PAD + index * (ITEM_H + gap),
    height: ITEM_H,
  };
}

/** The 50dp icon rail plus the gradient panel; `children` is the active section's content. */
export function DrawerShell({
  active,
  unreadNotifications,
  avatarUri,
  onSelect,
  children,
}: {
  active: MenuSection;
  unreadNotifications: boolean;
  avatarUri: string;
  onSelect: (section: MenuSection) => void;
  children: ReactNode;
}) {
  return (
    <View style={styles.root}>
      <View style={styles.rail}>
        {RAIL.map(item => {
          const on = item.key === active;
          return (
            <Pressable
              key={item.key}
              accessibilityRole="button"
              accessibilityLabel={t(item.label as 'MENU_DISCOVER')}
              accessibilityState={{ selected: on }}
              onPress={() => onSelect(item.key)}
              style={styles.item}
            >
              <Icon
                name={on ? item.on : item.off}
                size={24}
                color={on ? colors.lavender : colors.snow}
                strokeWidth={2}
                fill={on && item.filled ? colors.lavender : undefined}
              />
              {item.key === 'notifications' && unreadNotifications && (
                <View style={styles.dot}>
                  <Badge />
                </View>
              )}
              {on && <View style={styles.glow} />}
            </Pressable>
          );
        })}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('MENU_PROFILE')}
          onPress={() => onSelect('profile')}
          style={styles.item}
        >
          <View style={styles.profile}>
            <Avatar uri={avatarUri} size={u(21)} backdrop={colors.lavender} />
          </View>
        </Pressable>
      </View>
      <View style={styles.panel}>
        <LinearFill
          angle={135}
          stops={[
            { offset: 0, color: colors.purple350 },
            { offset: 1, color: colors.purple450 },
          ]}
        />
        {children}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    position: 'absolute',
    left: DRAWER_LEFT,
    top: DRAWER_TOP,
    height: DRAWER_H,
    flexDirection: 'row',
  },
  rail: {
    width: RAIL_W,
    justifyContent: 'space-between',
    paddingVertical: RAIL_PAD,
    borderTopLeftRadius: u(12),
    borderBottomLeftRadius: u(12),
    backgroundColor: 'rgba(36,3,59,0.92)',
  },
  item: { height: ITEM_H, alignItems: 'center', justifyContent: 'center' },
  dot: { position: 'absolute', top: u(6), right: u(10) },
  glow: {
    position: 'absolute',
    right: 0,
    width: u(2),
    height: u(28),
    backgroundColor: 'rgba(232,185,255,0.45)',
  },
  profile: {
    padding: u(1),
    borderRadius: u(12),
    borderWidth: 1,
    borderColor: colors.snow,
  },
  panel: {
    width: PANEL_W,
    borderTopRightRadius: u(12),
    borderBottomRightRadius: u(12),
    overflow: 'hidden',
  },
});
