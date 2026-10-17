import { StyleSheet, View } from 'react-native';
import { IconButton, Pill } from '../ds';
import { t } from '../i18n';
import { Mode } from './backpackState';
import { u } from '../ds/scale';

/** Wearables / Emotes switch and the filter button; the filter menu is rendered by the screen. */
export function BackpackHeader({
  mode,
  filterActive,
  menuOpen,
  onMode,
  onFilter,
  filterGap = 22,
}: {
  mode: Mode;
  /** A non-default filter is applied: the button gets the red dot. */
  filterActive: boolean;
  menuOpen: boolean;
  onMode: (mode: Mode) => void;
  onFilter: () => void;
  /** Space before the filter button (dp): wider in portrait than in landscape. */
  filterGap?: number;
}) {
  return (
    <View style={styles.row}>
      <Pill
        label={t('BACKPACK_WEARABLES')}
        icon="wearables"
        selected={mode === 'wearables'}
        onPress={() => onMode('wearables')}
      />
      <Pill
        label={t('BACKPACK_EMOTES')}
        icon="emotes"
        selected={mode === 'emotes'}
        onPress={() => onMode('emotes')}
      />
      <View style={{ marginLeft: u(filterGap - 12) }}>
        <IconButton
          icon="filter"
          label={t('BACKPACK_FILTER')}
          active={menuOpen}
          badge={filterActive}
          onPress={onFilter}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: u(12) },
});
