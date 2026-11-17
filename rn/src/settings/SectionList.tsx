import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, MenuNavItem, u } from '@dcl/ui-native';
import { t } from '../i18n';
import { sectionsFor, SectionKey } from './sections';

/** The section rows (`Menu/NavItem`): Large in the portrait list, Small in the landscape column,
 * where `selected` marks the open one. */
export function SectionRows({
  devTools,
  selected,
  size,
  onSelect,
}: {
  devTools: boolean;
  selected?: SectionKey | null;
  size: 'Large' | 'Small';
  onSelect: (key: SectionKey) => void;
}) {
  return (
    <View style={size === 'Large' ? styles.listRows : styles.tightRows}>
      {sectionsFor(devTools).map(s => (
        <MenuNavItem
          key={s.key}
          label={t(s.label)}
          size={size}
          selected={selected === s.key}
          onPress={() => onSelect(s.key)}
        />
      ))}
    </View>
  );
}

/** "Version 1.14.0" with a copy icon. */
export function VersionRow({
  version,
  onCopy,
}: {
  version: string;
  onCopy: () => void;
}) {
  return (
    <View style={styles.version}>
      <AppText variant="caption" style={styles.versionText}>
        {t('SETTINGS_VERSION', { version })}
      </AppText>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('SETTINGS_COPY_VERSION')}
        hitSlop={10}
        onPress={onCopy}
      >
        <Icon name="copy" size={18} color={colors.lavender} />
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  listRows: { gap: u(15) },
  tightRows: { gap: u(4) },
  version: { flexDirection: 'row', alignItems: 'center', gap: u(8) },
  versionText: { color: colors.lavender, fontSize: u(12) },
});
