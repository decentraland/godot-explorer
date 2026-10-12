import { AvatarState } from '../events';
import { Wearable } from './catalog';
import { Slot, TabKey } from './taxonomy';

export type ItemFilter = 'all' | 'collectibles';

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

export const isEquipped = (avatar: AvatarState | null, urn: string): boolean =>
  avatar !== null &&
  (same(avatar.body_shape, urn) ||
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
      item.bodyShapes.some(shape => same(shape, avatar.body_shape))
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

/** The thumbnail of whatever the avatar wears in a slot, for the slot tile. */
export function equippedThumbnail(
  slot: Slot,
  catalog: Wearable[],
  avatar: AvatarState | null,
): string | undefined {
  if (!slot.category) {
    return undefined;
  }
  return catalog.find(
    item => item.category === slot.category && isEquipped(avatar, item.id),
  )?.thumbnail;
}

/** Index of the emote slot to fill: the selected one, else the first empty one, else the selected. */
export function targetEmoteSlot(emotes: string[], selected: number): number {
  if (emotes[selected] === '') {
    return selected;
  }
  const empty = emotes.indexOf('');
  return empty === -1 ? selected : empty;
}
