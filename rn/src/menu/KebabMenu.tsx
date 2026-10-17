import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, IconName, u } from '../ds';
import { t } from '../i18n';

export type KebabAction = 'share' | 'reportContent' | 'reportBug';

const ITEMS: { action: KebabAction; icon: IconName; label: string }[] = [
  { action: 'share', icon: 'share', label: 'MENU_KEBAB_SHARE' },
  { action: 'reportContent', icon: 'flag', label: 'MENU_KEBAB_REPORT_CONTENT' },
  { action: 'reportBug', icon: 'bug', label: 'MENU_KEBAB_REPORT_BUG' },
];

/** Popover under the header's three dots; the caller dismisses it on an outside tap. */
export function KebabMenu({
  onSelect,
}: {
  onSelect: (action: KebabAction) => void;
}) {
  return (
    <View style={styles.menu} accessibilityRole="menu">
      {ITEMS.map(item => (
        <Pressable
          key={item.action}
          accessibilityRole="menuitem"
          accessibilityLabel={t(item.label as 'MENU_KEBAB_SHARE')}
          onPress={() => onSelect(item.action)}
          style={({ pressed }) => [styles.row, pressed && styles.pressed]}
        >
          <Icon
            name={item.icon}
            size={16}
            color={colors.snow}
            strokeWidth={2}
          />
          <AppText variant="button" style={styles.label}>
            {t(item.label as 'MENU_KEBAB_SHARE')}
          </AppText>
        </Pressable>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  menu: {
    width: u(159),
    paddingVertical: u(0),
    borderRadius: u(6),
    borderWidth: 0.5,
    borderColor: 'rgba(232,185,255,0.3)',
    backgroundColor: '#1B0030',
    overflow: 'hidden',
  },
  row: {
    height: u(36),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(10),
    paddingHorizontal: u(20),
  },
  pressed: { backgroundColor: colors.white10 },
  label: { fontFamily: 'Inter-Medium', fontSize: u(15) },
});
