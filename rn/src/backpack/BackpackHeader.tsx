import { StyleSheet, View } from 'react-native';
import { IconButton, PillButton, space, u } from '@dcl/ui-native';
import { t } from '../i18n';
import { Mode } from './backpackState';

/** Wearables / Emotes switch and the filter button; the filter menu is rendered by the screen. */
export function BackpackHeader({
  mode,
  menuOpen,
  filtered = false,
  onMode,
  onFilter,
  filterGap = u(22),
}: {
  mode: Mode;
  menuOpen: boolean;
  /** A filter other than All is applied: the DS filter button's badge dot. */
  filtered?: boolean;
  onMode: (mode: Mode) => void;
  onFilter: () => void;
  /** Space from the Emotes pill to the filter button: wider in portrait than in landscape. */
  filterGap?: number;
}) {
  return (
    <View style={styles.row}>
      <PillButton
        big
        color="Ruby"
        label={t('BACKPACK_WEARABLES')}
        icon="wearables"
        selected={mode === 'wearables'}
        onPress={() => onMode('wearables')}
      />
      <PillButton
        big
        color="Ruby"
        label={t('BACKPACK_EMOTES')}
        icon="emotes"
        selected={mode === 'emotes'}
        onPress={() => onMode('emotes')}
      />
      <View style={{ marginLeft: filterGap - space.lg }}>
        <IconButton
          icon="filter"
          accessibilityLabel={t('BACKPACK_FILTER')}
          selected={menuOpen}
          badge={filtered}
          onPress={onFilter}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: space.lg },
});
