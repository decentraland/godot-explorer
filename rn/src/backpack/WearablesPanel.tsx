import { memo, startTransition, useEffect, useRef, useState } from 'react';
import { ScrollView, StyleSheet, View } from 'react-native';
import {
  AppText,
  colors,
  ItemCard,
  PillButton,
  SlotTile,
  Tabs,
  layout,
  space,
  u,
  useSheetDrag,
} from '@dcl/ui-native';
import type { AvatarState } from '../core/native';
import { t } from '../i18n';
import type { I18nKey } from '../i18n';
import { Wearable } from './catalog';
import type { SubRow } from './BackpackChrome';
import { EmptyBlock, SkeletonGrid } from './GridStates';
import { ItemGrid } from './ItemGrid';
import { KeptLayer } from './KeptLayer';
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
  /** Kept mounted behind the visible tab: it skips renders until it shows again. */
  hidden?: boolean;
  /** The tab the row highlights, ahead of `tab` while the new grid renders. */
  activeTab?: TabKey;
  slot: Slot | null;
  filter: ItemFilter;
  columns?: number;
  bottomInset?: number;
  /** Item whose equip is in flight; its tile is dimmed. */
  busy?: string | null;
  copy: EmptyCopy;
  subRow?: SubRow;
  /** The entry flow's editor: 62 dp category tiles and no colour row, as Figma AVATAR_CUSTOMIZE draws it. */
  entry?: boolean;
  onTab: (tab: TabKey) => void;
  onSlot: (slot: Slot) => void;
  onToggle: (item: Wearable, equipped: boolean) => void;
  onColor: (target: ColorTarget, color: string) => void;
  onOpenColor: (target: ColorTarget) => void;
  onMarketplace: () => void;
};

/** Tabs, then the slot row, inline swatches and item grid of the Wearables mode. */
export const WearablesPanel = memo(
  WearablesPanelView,
  (prev, next) => !!prev.hidden && !!next.hidden,
);

function WearablesPanelView(props: WearablesPanelProps) {
  const { avatar, tab, slot } = props;
  const hidden = !!props.hidden;
  // "All" orders by what was worn when the tab opened, so a tile never jumps away from the finger
  // on equip or unequip (as in 1.x).
  const order = useRef<{
    tab: TabKey;
    avatar: AvatarState | null;
    hidden: boolean;
  } | null>(null);
  if (
    order.current?.tab !== tab ||
    order.current.avatar === null ||
    (order.current.hidden && !hidden)
  ) {
    order.current = { tab, avatar, hidden };
  }
  order.current.hidden = hidden;
  const drag = useSheetDrag();
  const pills = props.subRow === 'pills';

  // Once the tab shows, its other slots' grids mount hidden at low priority: picking a slot then
  // shows a grid whose cards and images are already there.
  const [kept, setKept] = useState<string[]>([]);
  useEffect(() => {
    if (hidden || tab === 'all') {
      return undefined;
    }
    const frame = requestAnimationFrame(() =>
      startTransition(() => setKept(SLOTS[tab].map(entry => entry.key))),
    );
    return () => cancelAnimationFrame(frame);
  }, [hidden, tab]);
  const grids: (Slot | null)[] =
    tab === 'all'
      ? [null]
      : SLOTS[tab].filter(
          entry => entry.key === slot?.key || kept.includes(entry.key),
        );

  return (
    <View style={styles.fill}>
      <View
        style={[styles.tabs, tab !== 'all' && !pills && styles.tabsAboveSlots]}
        {...drag}
      >
        <Tabs
          items={TABS.map(entry => ({ key: entry.key, label: t(entry.label) }))}
          active={props.activeTab ?? tab}
          onChange={props.onTab}
        />
      </View>
      <View style={styles.fill}>
        {grids.map(entry => {
          const shown = entry?.key === slot?.key;
          return (
            <KeptLayer key={entry?.key ?? 'all'} shown={shown}>
              <SlotGrid
                {...props}
                hidden={hidden || !shown}
                gridSlot={entry}
                wornFirst={order.current?.avatar ?? avatar}
              />
            </KeptLayer>
          );
        })}
      </View>
    </View>
  );
}

type SlotGridProps = WearablesPanelProps & {
  /** The slot this grid lists (null under All); `slot` is the one selected in the row. */
  gridSlot: Slot | null;
  wornFirst: AvatarState | null;
};

/** One slot's row of slots, swatches and items; hidden, it skips renders until it shows again. */
const SlotGrid = memo(
  SlotGridView,
  (prev, next) => !!prev.hidden && !!next.hidden,
);

function SlotGridView(props: SlotGridProps) {
  const { catalog, avatar, tab, gridSlot, filter, columns, bottomInset, busy } =
    props;
  const hidden = !!props.hidden;
  const items = catalog
    ? gridItems(tab, gridSlot, catalog, avatar, filter, props.wornFirst)
    : [];
  const pills = props.subRow === 'pills';
  // The pill generation also colours the body shape; with nothing to show the swatch row goes too,
  // except for skin, whose colour applies to the body even when no skin item is owned.
  const color =
    gridSlot?.color ??
    (pills && gridSlot?.key === 'shape' ? ('skin' as ColorTarget) : undefined);
  const showSwatches = !catalog || items.length > 0 || color === 'skin';
  const palette = color ? PALETTES[colorKey(color)] : [];
  const value = color && avatar ? avatar[colorKey(color)] : '';
  const selected = props.slot?.key;

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
            <PillButton
              key={entry.key}
              label={t(PILL_LABELS[entry.key] ?? entry.label)}
              selected={selected === entry.key}
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
          contentContainerStyle={props.entry ? styles.entrySlots : styles.slots}
        >
          {SLOTS[tab].map(entry => {
            const worn = catalog
              ? equippedItem(entry, catalog, avatar)
              : undefined;
            return props.entry ? (
              <View key={entry.key} style={styles.entrySlot}>
                <ItemCard
                  style={styles.entryCard}
                  label={t(entry.label)}
                  thumbnail={worn?.thumbnail}
                  empty={!worn}
                  state={selected === entry.key ? 'selected' : 'default'}
                  onPress={() => props.onSlot(entry)}
                />
                <AppText variant="body" style={styles.entryLabel}>
                  {t(entry.label)}
                </AppText>
              </View>
            ) : (
              <SlotTile
                key={entry.key}
                label={t(entry.label)}
                thumbnail={worn?.thumbnail}
                selected={selected === entry.key}
                onPress={() => props.onSlot(entry)}
              />
            );
          })}
        </ScrollView>
      )}
      {color && avatar && showSwatches && !props.entry ? (
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
    <ItemGrid
      items={items}
      keyOf={item => item.id}
      // A hidden grid keeps only its first rows ready.
      initialNumToRender={hidden ? 9 : undefined}
      windowSize={hidden ? 1 : undefined}
      columns={columns}
      bottomInset={bottomInset}
      // All has nothing above its first row: room for the equipped ring drawn outside the cards.
      overhang={tab === 'all' ? space['2xs'] : 0}
      header={header}
      empty={
        catalog ? (
          <EmptyBlock
            icon="search"
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
              rarity={item.rarity}
              state={equipped ? 'unequip' : 'default'}
              actionLabel={
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
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  /** `Header`: 8 px above the tabs, 32 px down to the slots or the grid. */
  tabs: {
    paddingHorizontal: layout.gutter,
    paddingTop: space.xs,
    paddingBottom: space.xl,
  },
  /** The slot row scrolls and would clip the selected ring drawn outside the card: it gets 3 dp
   * of room on top, taken from the tabs' gap. */
  tabsAboveSlots: { paddingBottom: space['26'] },
  slotsScroll: { flexGrow: 0 },
  slots: {
    paddingHorizontal: layout.gutter,
    gap: space.xl,
    paddingTop: space['2xs'],
    paddingBottom: space.xl,
  },
  /** `Presets` sits 32 px under the slots and 54 px above the grid; the strip pads its rings. */
  swatches: {
    marginTop: -space['2xs'],
    paddingBottom: space['54'] - space['2xs'],
  },
  /** Figma AVATAR_CUSTOMIZE `Categories`: 124 px `Cards-Wearables` (emote wheel) tiles 32 px apart, the
   * 24 px label 4 px under each, 40 px above the cards. */
  entrySlots: {
    paddingHorizontal: layout.gutter,
    paddingTop: u(4),
    gap: u(16),
    paddingBottom: u(20),
  },
  entrySlot: { width: u(62), alignItems: 'center', gap: u(2) },
  entryCard: { width: u(62), height: u(62) },
  entryLabel: { fontSize: u(12), lineHeight: u(14.5), color: colors.snow },
  busy: { opacity: 0.55 },
  pills: {
    paddingHorizontal: layout.gutter,
    gap: u(8),
    paddingBottom: u(14),
    alignItems: 'center',
  },
});
