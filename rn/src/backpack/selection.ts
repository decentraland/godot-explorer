import { Control } from '../core/native';
import type { AvatarState } from '../core/native';
import { Wearable } from './catalog';
import { isRemovable, Slot, TabKey } from './taxonomy';

export type ItemFilter = 'all' | 'collectibles';

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

export const isEquipped = (avatar: AvatarState | null, urn: string): boolean =>
  avatar !== null &&
  (same(avatar.bodyShape, urn) ||
    avatar.wearables.some(worn => same(worn, urn)));

/** Wearables the current body shape can wear; the body shapes themselves are always listed. */
export function visibleWearables(
  catalog: Wearable[],
  avatar: AvatarState | null,
  filter: ItemFilter,
): Wearable[] {
  return catalog.filter(item => {
    if (filter === 'collectibles' && !item.collectible) {
      return false;
    }
    if (item.category === 'body_shape' || !avatar) {
      return true;
    }
    return (
      item.bodyShapes.length === 0 ||
      item.bodyShapes.some(shape => same(shape, avatar.bodyShape))
    );
  });
}

/** Items for the grid under a tab; "All" lists everything the avatar wears first. */
export function gridItems(
  tab: TabKey,
  slot: Slot | null,
  catalog: Wearable[],
  avatar: AvatarState | null,
  filter: ItemFilter,
): Wearable[] {
  const visible = visibleWearables(catalog, avatar, filter);
  if (tab === 'all') {
    return [
      ...visible.filter(item => isEquipped(avatar, item.id)),
      ...visible.filter(item => !isEquipped(avatar, item.id)),
    ];
  }
  if (!slot?.category) {
    return [];
  }
  return visible.filter(item => item.category === slot.category);
}

/** Whatever the avatar wears in a slot. */
export function equippedItem(
  slot: Slot,
  catalog: Wearable[],
  avatar: AvatarState | null,
): Wearable | undefined {
  return slot.category
    ? catalog.find(
        item => item.category === slot.category && isEquipped(avatar, item.id),
      )
    : undefined;
}

/** The thumbnail of whatever the avatar wears in a slot, for the slot tile. */
export const equippedThumbnail = (
  slot: Slot,
  catalog: Wearable[],
  avatar: AvatarState | null,
): string | undefined => equippedItem(slot, catalog, avatar)?.thumbnail;

/** Index of the emote slot to fill: the selected one, else the first empty one, else the selected. */
export function targetEmoteSlot(emotes: string[], selected: number): number {
  if (emotes[selected] === '') {
    return selected;
  }
  const empty = emotes.indexOf('');
  return empty === -1 ? selected : empty;
}

/** What tapping a wearable sends; null when nothing should (the required parts can't be removed). */
export function wearableCommand(
  item: Wearable,
  equipped: boolean,
): Control | null {
  if (item.category === 'body_shape') {
    return equipped ? null : new Control.SetBodyShape({ urn: item.id });
  }
  if (equipped) {
    return isRemovable(item.category)
      ? new Control.Unequip({ urn: item.id })
      : null;
  }
  return new Control.Equip({ urn: item.id });
}

/** The UNEQUIP strip is drawn on the body shape too (Figma), though tapping it does nothing. */
export const showsUnequipStrip = (category: string): boolean =>
  category === 'body_shape' || isRemovable(category);

/** True when the colour is none of the palette's presets, i.e. the rainbow wheel is the selected swatch. */
export const isCustomColor = (palette: string[], value: string): boolean =>
  !palette.some(color => same(color, value));

/** Emotes the Collectibles filter keeps: the ones with a rarity. */
export function visibleEmotes<T extends { collectible?: boolean }>(
  emotes: T[],
  filter: ItemFilter,
): T[] {
  return filter === 'collectibles'
    ? emotes.filter(emote => emote.collectible)
    : emotes;
}
