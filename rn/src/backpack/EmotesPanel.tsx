import { Image, Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { GradientView, Icon, ItemCard, colors, layout, radii } from '../ds';
import { Emote, emoteIdOf } from './emotes';
import { ItemGrid } from './ItemGrid';
import { u } from '../ds/scale';

const SLOT_SIZE = u(56);

function EmoteThumb({ emote }: { emote?: Emote }) {
  return emote?.thumbnail ? (
    <Image
      source={{ uri: emote.thumbnail }}
      style={StyleSheet.absoluteFill}
      resizeMode="cover"
    />
  ) : (
    <View style={styles.silhouette}>
      <Icon name="emotes" size={26} color={colors.shadow} />
    </View>
  );
}

/** The ten equipped emote slots, shown above the sheet in Emotes mode. */
export function EmoteSlotsRow({
  slots,
  emotes,
  selected,
  onSelect,
}: {
  slots: string[];
  emotes: Emote[];
  selected: number;
  onSelect: (slot: number) => void;
}) {
  return (
    <ScrollView
      removeClippedSubviews={false}
      horizontal
      showsHorizontalScrollIndicator={false}
      style={styles.slotsScroll}
      contentContainerStyle={styles.slotsRow}
    >
      {slots.map((urn, index) => {
        const emote = urn
          ? emotes.find(entry => entry.id === emoteIdOf(urn))
          : undefined;
        return (
          <Pressable
            key={index}
            accessibilityRole="button"
            accessibilityLabel={`Emote slot ${index + 1}`}
            accessibilityState={{ selected: selected === index }}
            onPress={() => onSelect(index)}
            style={[styles.slot, selected === index && styles.slotSelected]}
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
              <View style={[StyleSheet.absoluteFill, styles.emptySlot]}>
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
      })}
    </ScrollView>
  );
}

/** Grid of every emote; tapping plays it on the avatar and puts it in the selected slot. */
export function EmotesGrid({
  emotes,
  slots,
  bottomInset,
  onPress,
}: {
  emotes: Emote[];
  slots: string[];
  bottomInset?: number;
  onPress: (emote: Emote, equipped: boolean) => void;
}) {
  const worn = new Set(slots.filter(Boolean).map(emoteIdOf));
  return (
    <ItemGrid
      items={emotes}
      keyOf={emote => emote.id}
      bottomInset={bottomInset}
      header={<View style={styles.gridTop} />}
      renderItem={emote => {
        const equipped = worn.has(emote.id);
        return (
          <View>
            <ItemCard
              label={emote.name}
              equipped={equipped}
              caption="Unequip"
              thumbnail={emote.thumbnail}
              onPress={() => onPress(emote, equipped)}
            />
            {!emote.thumbnail && (
              <View pointerEvents="none" style={StyleSheet.absoluteFill}>
                <EmoteThumb />
              </View>
            )}
          </View>
        );
      }}
    />
  );
}

const styles = StyleSheet.create({
  slotsScroll: { flexGrow: 0 },
  slotsRow: {
    paddingHorizontal: layout.gutter,
    gap: u(8),
    paddingVertical: u(6),
  },
  slot: {
    width: SLOT_SIZE,
    height: SLOT_SIZE,
    borderRadius: radii.card,
    overflow: 'hidden',
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.8)',
  },
  slotSelected: { borderWidth: 2, borderColor: colors.orange },
  emptySlot: {
    backgroundColor: colors.purple550,
    alignItems: 'center',
    justifyContent: 'center',
  },
  silhouette: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  gridTop: { height: u(12) },
});
