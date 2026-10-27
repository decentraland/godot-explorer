import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, LinkRow, u } from '../ds';
import { t } from '../i18n';
import { sectionsFor, SectionKey } from './sections';

/** The section rows; `selected` marks the open one in the landscape column. */
export function SectionRows({
  devTools,
  selected,
  rowHeight,
  onSelect,
}: {
  devTools: boolean;
  selected?: SectionKey | null;
  rowHeight: number;
  onSelect: (key: SectionKey) => void;
}) {
  return (
    <View style={rowHeight < 40 ? styles.tightRows : undefined}>
      {sectionsFor(devTools).map(s => (
        <LinkRow
          key={s.key}
          label={t(s.label)}
          height={rowHeight}
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
  tightRows: { gap: u(4) },
  version: { flexDirection: 'row', alignItems: 'center', gap: u(8) },
  versionText: { color: colors.lavender, fontSize: u(12) },
});
