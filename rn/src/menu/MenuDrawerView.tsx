import { ReactNode } from 'react';
import { Pressable, StyleSheet, useWindowDimensions, View } from 'react-native';
import { t } from '../i18n';
import { u } from '../ds';
import {
  DRAWER_H,
  DRAWER_LEFT,
  DRAWER_TOP,
  DrawerShell,
  PANEL_W,
  RAIL_W,
} from './DrawerShell';
import { closeZones, MenuSection } from './drawerState';

/** Gamepad bounding box, in dp from the right/bottom edges (Figma 1205-1500 x 385-672 of 1600x720). */
const PAD_W = u(198);
const PAD_TOP = u(190);

/**
 * Full-screen, touch-transparent host: only the drawer and the close zones take touches, so the
 * world and the gamepad keep working around the panel.
 */
export function MenuDrawerView({
  section,
  unreadNotifications,
  avatarUri = '',
  onSelect,
  onClose,
  children,
  overlay,
}: {
  section: MenuSection;
  unreadNotifications: boolean;
  avatarUri?: string;
  onSelect: (section: MenuSection) => void;
  onClose: () => void;
  children: ReactNode;
  overlay?: ReactNode;
}) {
  const { width, height } = useWindowDimensions();
  const panel = {
    left: DRAWER_LEFT,
    top: DRAWER_TOP,
    right: DRAWER_LEFT + RAIL_W + PANEL_W,
    bottom: DRAWER_TOP + DRAWER_H,
  };
  const zones = closeZones({ width, height }, panel, {
    left: width - PAD_W,
    top: PAD_TOP,
    right: width,
    bottom: height,
  });
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="box-none">
      {zones.map((zone, i) => (
        <Pressable
          key={i}
          accessibilityLabel={t('MENU_CLOSE')}
          onPress={onClose}
          style={[styles.zone, zone]}
        />
      ))}
      <DrawerShell
        active={section}
        unreadNotifications={unreadNotifications}
        avatarUri={avatarUri}
        onSelect={onSelect}
      >
        {children}
      </DrawerShell>
      {overlay}
    </View>
  );
}

const styles = StyleSheet.create({ zone: { position: 'absolute' } });
