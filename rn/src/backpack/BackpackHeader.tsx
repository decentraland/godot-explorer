import { StyleSheet, View } from 'react-native';
import { IconButton, PillButton, u } from '@dcl/ui-native';
import { t } from '../i18n';
import { Mode } from './backpackState';

/** Wearables / Emotes switch and the filter button; the filter menu is rendered by the screen. */
export function BackpackHeader({
  mode,
  menuOpen,
  onMode,
  onFilter,
  filterGap = 22,
}: {
  mode: Mode;
  menuOpen: boolean;
  onMode: (mode: Mode) => void;
  onFilter: () => void;
  /** Space before the filter button (dp): wider in portrait than in landscape. */
  filterGap?: number;
}) {
  return (
    <View style={styles.row}>
      <PillButton
        big
        label={t('BACKPACK_WEARABLES')}
        icon="wearables"
        selected={mode === 'wearables'}
        onPress={() => onMode('wearables')}
      />
      <PillButton
        big
        label={t('BACKPACK_EMOTES')}
        icon="emotes"
        selected={mode === 'emotes'}
        onPress={() => onMode('emotes')}
      />
      <View style={{ marginLeft: u(filterGap - 12) }}>
        <IconButton
          icon="filter"
          accessibilityLabel={t('BACKPACK_FILTER')}
          selected={menuOpen}
          onPress={onFilter}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: u(12) },
});
