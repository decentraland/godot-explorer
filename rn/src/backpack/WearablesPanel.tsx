import { ScrollView, StyleSheet, View } from 'react-native';
import { AppText, ItemCard, SkeletonCard, SlotTile, Tabs, layout } from '../ds';
import { AvatarState } from '../events';
import { ColorSwatchButton } from './ColorSwatchButton';
import { Wearable } from './catalog';
import { ItemGrid } from './ItemGrid';
import {
  equippedThumbnail,
  gridItems,
  isEquipped,
  ItemFilter,
} from './selection';
import { isRemovable, SLOTS, Slot, TABS, TabKey } from './taxonomy';

export type WearablesPanelProps = {
  catalog: Wearable[] | null;
  avatar: AvatarState | null;
  tab: TabKey;
  slot: Slot | null;
  filter: ItemFilter;
  bottomInset?: number;
  onTab: (tab: TabKey) => void;
  onSlot: (slot: Slot) => void;
  onToggle: (item: Wearable, equipped: boolean) => void;
  onOpenColor: (slot: Slot) => void;
};

/** Tabs, category slots and the item grid of the Wearables mode. */
export function WearablesPanel(props: WearablesPanelProps) {
  const {
    catalog,
    avatar,
    tab,
    slot,
    filter,
    bottomInset,
    onTab,
    onSlot,
    onToggle,
    onOpenColor,
  } = props;
  const items = catalog ? gridItems(tab, slot, catalog, avatar, filter) : [];

  const header = (
    <View>
      <View style={styles.tabs}>
        <Tabs items={TABS} active={tab} onChange={onTab} />
      </View>
      {tab !== 'all' && (
        <ScrollView
          horizontal
          showsHorizontalScrollIndicator={false}
          style={styles.slotsScroll}
          contentContainerStyle={styles.slots}
        >
          {SLOTS[tab].map(entry => (
            <SlotTile
              key={entry.key}
              label={entry.label}
              thumbnail={
                catalog ? equippedThumbnail(entry, catalog, avatar) : undefined
              }
              selected={slot?.key === entry.key}
              onPress={() => onSlot(entry)}
            />
          ))}
        </ScrollView>
      )}
      {slot?.color && slot.category && avatar && (
        <View style={styles.colorRow}>
          <AppText variant="caption">Color</AppText>
          <ColorSwatchButton
            color={avatar[slot.color]}
            onPress={() => onOpenColor(slot)}
          />
        </View>
      )}
    </View>
  );

  if (!catalog) {
    return (
      <View style={styles.fill}>
        {header}
        <View style={styles.skeleton}>
          {Array.from({ length: 6 }, (_, index) => (
            <View key={index} style={styles.skeletonCell}>
              <SkeletonCard />
            </View>
          ))}
        </View>
      </View>
    );
  }

  return (
    <ItemGrid
      items={items}
      keyOf={item => item.id}
      bottomInset={bottomInset}
      header={header}
      empty={
        <View style={styles.empty}>
          <AppText variant="caption">Nothing here yet</AppText>
        </View>
      }
      renderItem={item => {
        const equipped = isEquipped(avatar, item.id);
        return (
          <ItemCard
            label={item.name}
            thumbnail={item.thumbnail}
            rarity={item.rarity}
            equipped={equipped}
            caption={isRemovable(item.category) ? 'Unequip' : undefined}
            onPress={() => onToggle(item, equipped)}
          />
        );
      }}
    />
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  tabs: { paddingHorizontal: layout.gutter, paddingTop: 8, paddingBottom: 12 },
  slotsScroll: { flexGrow: 0 },
  slots: { paddingHorizontal: layout.gutter - 3, gap: 8, paddingBottom: 14 },
  colorRow: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'flex-end',
    gap: 10,
    paddingHorizontal: layout.gutter,
    paddingBottom: 12,
  },
  skeleton: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    paddingHorizontal: layout.gutter,
    gap: layout.gap,
  },
  skeletonCell: { width: '31%' },
  empty: { alignItems: 'center', paddingTop: 32 },
});
