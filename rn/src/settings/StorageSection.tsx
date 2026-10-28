import { StyleSheet, View } from 'react-native';
import {
  AppText,
  colors,
  Dropdown,
  fonts,
  OutlinedButton,
  ProgressBar,
  u,
} from '../ds';
import { t } from '../i18n';
import { cacheFraction, formatCacheSize, formatMaxCache } from './format';
import { CACHE_SIZES_GB } from './keys';
import { Group, SectionCaption } from './rows';
import type { SettingsModel } from './useSettings';

export function StorageSection({
  m,
  defaultOpen = false,
}: {
  m: SettingsModel;
  defaultOpen?: boolean;
}) {
  const maxGb = CACHE_SIZES_GB[m.values.max_cache_size] ?? CACHE_SIZES_GB[1];
  return (
    <View>
      <SectionCaption title={t('SETTINGS_TEMPORARY_FILES')} gap={14} />
      <Group>
        <View>
          <View style={styles.sizeRow}>
            <AppText variant="body" style={styles.label}>
              {t('SETTINGS_CURRENT_CACHE_SIZE')}
            </AppText>
            <AppText variant="body" style={styles.value}>
              {formatCacheSize(m.cacheBytes)}
            </AppText>
          </View>
          <ProgressBar
            label={t('SETTINGS_CURRENT_CACHE_SIZE')}
            radius={3}
            height={20}
            value={cacheFraction(m.cacheBytes, maxGb)}
          />
        </View>
        <Dropdown
          variant="settings"
          label={t('SETTINGS_MAX_CACHE_SIZE')}
          helper={t('SETTINGS_SETS_HOW_MUCH_STORAGE_THE_APP')}
          value={String(m.values.max_cache_size)}
          options={CACHE_SIZES_GB.map((gb, i) => ({
            value: String(i),
            label: formatMaxCache(gb),
          }))}
          defaultOpen={defaultOpen}
          onChange={v => m.set('max_cache_size', Number(v))}
        />
        <View>
          <OutlinedButton
            box
            label={t('SETTINGS_CLEAN_CACHE')}
            disabled={m.cacheBytes === 0}
            onPress={() => {
              m.clearCache();
            }}
          />
          <AppText variant="caption" style={styles.helper}>
            {t('SETTINGS_CLEARS_DOWNLOADED_FILES_USED_TO_SPEED')}
          </AppText>
        </View>
      </Group>
    </View>
  );
}

const styles = StyleSheet.create({
  sizeRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: u(12),
  },
  label: { fontFamily: fonts.medium, fontSize: u(15), color: colors.snow },
  value: { fontSize: u(15), color: colors.snow },
  helper: { marginTop: u(6), color: colors.lavender70, fontSize: u(9.6) },
});
