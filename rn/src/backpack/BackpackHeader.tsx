import { StyleSheet, View } from 'react-native';
import { IconButton, Pill } from '../ds';
import { Mode } from './backpackState';
import { u } from '../ds/scale';

/** Wearables / Emotes switch and the filter button; the filter menu is rendered by the screen. */
export function BackpackHeader({
  mode,
  filterActive,
  onMode,
  onFilter,
}: {
  mode: Mode;
  filterActive: boolean;
  onMode: (mode: Mode) => void;
  onFilter: () => void;
}) {
  return (
    <View style={styles.row}>
      <Pill
        label="Wearables"
        icon="wearables"
        selected={mode === 'wearables'}
        onPress={() => onMode('wearables')}
      />
      <Pill
        label="Emotes"
        icon="emotes"
        selected={mode === 'emotes'}
        onPress={() => onMode('emotes')}
      />
      <IconButton
        icon="filter"
        label="Filter"
        active={filterActive}
        onPress={onFilter}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: u(12) },
});
