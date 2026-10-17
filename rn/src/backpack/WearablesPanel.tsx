import { ScrollView, StyleSheet, View } from 'react-native';
import { ItemCard, Pill, SlotTile, Tabs, layout } from '../ds';
import type { AvatarState } from '../core/native';
import { t } from '../i18n';
import type { I18nKey } from '../i18n';
import { Wearable } from './catalog';
import type { SubRow } from './BackpackChrome';
import { EmptyBlock, SkeletonGrid } from './GridStates';
import { ItemArt } from './ItemArt';
import { ItemGrid } from './ItemGrid';
import type { EmptyCopy } from './platformPolicy';
import {
  equippedItem,
  gridItems,
  isCustomColor,
  isEquipped,
  ItemFilter,
  showsUnequipStrip,
} from './selection';
import { SwatchStrip } from './SwatchStrip';
import {
  colorKey,
  ColorTarget,
  PALETTES,
  SLOTS,
  Slot,
  TABS,
  TabKey,
} from './taxonomy';
import { u } from '../ds/scale';

/** The pill generation spells two slot labels differently. */
const PILL_LABELS: Record<string, I18nKey> = {
  facial_hair: 'BACKPACK_PILL_FACIAL_HAIR',
  hands_wear: 'BACKPACK_PILL_HANDS',
};

export type WearablesPanelProps = {
  /** null while the catalog loads: the grid shows the skeleton. */
  catalog: Wearable[] | null;
  avatar: AvatarState | null;
  tab: TabKey;
  slot: Slot | null;
  filter: ItemFilter;
  columns?: number;
  bottomInset?: number;
  /** Item whose equip is in flight; its tile is dimmed. */
  busy?: string | null;
  copy: EmptyCopy;
  subRow?: SubRow;
  onTab: (tab: TabKey) => void;
  onSlot: (slot: Slot) => void;
  onToggle: (item: Wearable, equipped: boolean) => void;
  onColor: (target: ColorTarget, color: string) => void;
  onOpenColor: (target: ColorTarget) => void;
  onMarketplace: () => void;
};

/** Tabs, then the slot row, inline swatches and item grid of the Wearables mode. */
export function WearablesPanel(props: WearablesPanelProps) {
  const { catalog, avatar, tab, slot, filter, columns, bottomInset, busy } =
    props;
  const items = catalog ? gridItems(tab, slot, catalog, avatar, filter) : [];
  const pills = props.subRow === 'pills';
  // The pill generation also colours the body shape, and drops the row when the list is empty.
  const color =
    slot?.color ??
    (pills && slot?.key === 'shape' ? ('skin' as ColorTarget) : undefined);
  const showSwatches = !pills || !catalog || items.length > 0;
  const palette = color ? PALETTES[colorKey(color)] : [];
  const value = color && avatar ? avatar[colorKey(color)] : '';

  const header = (
    <View>
      {tab !== 'all' && pills && (
        <ScrollView
          removeClippedSubviews={false}
          horizontal
          showsHorizontalScrollIndicator={false}
          style={styles.slotsScroll}
          contentContainerStyle={styles.pills}
        >
          {SLOTS[tab].map(entry => (
            <Pill
              key={entry.key}
              tone="soft"
              label={t(PILL_LABELS[entry.key] ?? entry.label)}
              selected={slot?.key === entry.key}
              onPress={() => props.onSlot(entry)}
            />
          ))}
        </ScrollView>
      )}
      {tab !== 'all' && !pills && (
        <ScrollView
          removeClippedSubviews={false}
          horizontal
          showsHorizontalScrollIndicator={false}
          style={styles.slotsScroll}
          contentContainerStyle={styles.slots}
        >
          {SLOTS[tab].map(entry => {
            const worn = catalog
              ? equippedItem(entry, catalog, avatar)
              : undefined;
            return (
              <SlotTile
                key={entry.key}
                label={t(entry.label)}
                size={56}
                thumbnail={worn?.thumbnail}
                art={
                  worn ? (
                    <ItemArt category={worn.category} name={worn.name} />
                  ) : undefined
                }
                selected={slot?.key === entry.key}
                onPress={() => props.onSlot(entry)}
              />
            );
          })}
        </ScrollView>
      )}
      {color && avatar && showSwatches ? (
        <View style={styles.swatches}>
          <SwatchStrip
            palette={palette}
            value={value}
            custom={isCustomColor(palette, value)}
            disabled={catalog === null}
            onSelect={next => props.onColor(color, next)}
            onWheel={() => props.onOpenColor(color)}
          />
        </View>
      ) : null}
    </View>
  );

  return (
    <View style={styles.fill}>
      <View style={styles.tabs}>
        <Tabs
          items={TABS.map(entry => ({ key: entry.key, label: t(entry.label) }))}
          active={tab}
          onChange={props.onTab}
        />
      </View>
      <ItemGrid
        items={items}
        keyOf={item => item.id}
        columns={columns}
        bottomInset={bottomInset}
        header={header}
        empty={
          catalog ? (
            <EmptyBlock
              icon="wearables"
              copy={props.copy}
              onMarketplace={props.onMarketplace}
            />
          ) : (
            <SkeletonGrid columns={columns} />
          )
        }
        renderItem={item => {
          const equipped = isEquipped(avatar, item.id);
          return (
            <View style={busy === item.id && styles.busy}>
              <ItemCard
                label={item.name}
                thumbnail={item.thumbnail}
                art={<ItemArt category={item.category} name={item.name} />}
                rarity={item.rarity}
                equipped={equipped}
                caption={
                  showsUnequipStrip(item.category)
                    ? t('BACKPACK_UNEQUIP')
                    : undefined
                }
                onPress={() => props.onToggle(item, equipped)}
              />
            </View>
          );
        }}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  tabs: {
    paddingHorizontal: layout.gutter,
    paddingBottom: u(12),
  },
  slotsScroll: { flexGrow: 0 },
  slots: {
    paddingHorizontal: layout.gutter - 3,
    gap: u(16),
    paddingBottom: u(12),
  },
  swatches: { paddingBottom: u(12) },
  busy: { opacity: 0.55 },
  pills: {
    paddingHorizontal: layout.gutter,
    gap: u(8),
    paddingBottom: u(14),
    alignItems: 'center',
  },
});
