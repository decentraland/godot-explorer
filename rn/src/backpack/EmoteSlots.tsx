import { Image, ScrollView, StyleSheet, View } from 'react-native';
import { ItemCard, layout, space, u } from '@dcl/ui-native';
import { t } from '../i18n';
import { Emote, emoteIdOf } from './emotes';
import { EmoteArt } from './ItemArt';

const SLOT = 56;
/** Landscape tiles are smaller so the ten slots fit a 360dp-high viewport in two columns of five. */
const SLOT_COMPACT = 44;
const GRID_GAP = 6;

export function EmoteThumb({ emote }: { emote?: Emote }) {
  return emote?.thumbnail ? (
    <Image
      source={{ uri: emote.thumbnail }}
      style={StyleSheet.absoluteFill}
      resizeMode="cover"
    />
  ) : (
    <EmoteArt id={emote?.id ?? ''} />
  );
}

/**
 * The ten equipped emote slots: a horizontal row above the sheet in portrait, a vertical column
 * beside the grid in landscape: `Cards-Wearables` at 112 px, the selected one with the wheel's orange
 * ring, the equipped state right after an equip.
 */
export function EmoteSlots({
  slots,
  emotes,
  selected,
  flash = false,
  vertical = false,
  onSelect,
}: {
  slots: string[];
  emotes: Emote[];
  selected: number | null;
  flash?: boolean;
  vertical?: boolean;
  onSelect: (slot: number) => void;
}) {
  const tiles = slots.map((urn, index) => {
    const id = urn ? emoteIdOf(urn) : '';
    const emote = id
      ? emotes.find(entry => entry.id === id) ?? { id, urn, name: id }
      : undefined;
    const on = selected === index;
    return (
      <ItemCard
        key={index}
        label={t('BACKPACK_EMOTE_SLOT', { number: index + 1 })}
        thumbnail={emote?.thumbnail}
        art={emote ? <EmoteArt id={emote.id} /> : undefined}
        empty={!emote}
        state={on ? (flash ? 'equipped' : 'selected') : 'default'}
        onPress={() => onSelect(index)}
        style={vertical ? styles.slotCompact : styles.slot}
      />
    );
  });
  return vertical ? (
    <View style={styles.grid}>
      <View style={styles.gridColumn}>{tiles.slice(0, 5)}</View>
      <View style={styles.gridColumn}>{tiles.slice(5)}</View>
    </View>
  ) : (
    <ScrollView
      removeClippedSubviews={false}
      horizontal
      showsHorizontalScrollIndicator={false}
      style={styles.scroll}
      contentContainerStyle={styles.row}
    >
      {tiles}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  scroll: { flexGrow: 0 },
  row: {
    paddingHorizontal: layout.gutter,
    gap: space.md,
    paddingVertical: space.xs,
  },
  grid: { flexDirection: 'row', gap: u(GRID_GAP) },
  gridColumn: { gap: u(GRID_GAP) },
  slot: { width: u(SLOT) },
  slotCompact: { width: u(SLOT_COMPACT) },
});
