import { Image, Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { GradientView, Icon, colors, layout, radii } from '../ds';
import { t } from '../i18n';
import { Emote, emoteIdOf } from './emotes';
import { EmoteArt } from './ItemArt';
import { u } from '../ds/scale';

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
 * beside the grid in landscape. The selected slot has the orange border, white right after an equip.
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
      <Pressable
        key={index}
        accessibilityRole="button"
        accessibilityLabel={t('BACKPACK_EMOTE_SLOT', { number: index + 1 })}
        accessibilityState={{ selected: on }}
        onPress={() => onSelect(index)}
        style={[
          styles.slot,
          vertical && styles.slotCompact,
          on && styles.slotSelected,
          on && flash && styles.slotFlash,
        ]}
      >
        {emote ? (
          <GradientView
            angle={225}
            stops={[
              { offset: 0, color: colors.cardLight },
              { offset: 0.65, color: colors.snow },
            ]}
            style={StyleSheet.absoluteFill}
          >
            <EmoteThumb emote={emote} />
          </GradientView>
        ) : (
          <View style={[StyleSheet.absoluteFill, styles.empty]}>
            <Icon
              name="plus"
              size={16}
              color={colors.silver}
              strokeWidth={1.6}
            />
          </View>
        )}
      </Pressable>
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
  row: { paddingHorizontal: layout.gutter, gap: u(8), paddingVertical: u(4) },
  grid: { flexDirection: 'row', gap: u(GRID_GAP) },
  gridColumn: { gap: u(GRID_GAP) },
  slot: {
    width: u(SLOT),
    height: u(SLOT),
    borderRadius: radii.card,
    overflow: 'hidden',
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.8)',
  },
  slotCompact: { width: u(SLOT_COMPACT), height: u(SLOT_COMPACT) },
  slotSelected: { borderWidth: 2, borderColor: colors.orange },
  slotFlash: { borderColor: colors.snow },
  empty: {
    backgroundColor: colors.purple550,
    alignItems: 'center',
    justifyContent: 'center',
  },
});
