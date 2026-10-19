import type { Snap } from '../ds';
import { SLOTS, Slot, TabKey, ColorTarget } from './taxonomy';
import { ItemFilter } from './selection';

export type Mode = 'wearables' | 'emotes';

export type BackpackState = {
  mode: Mode;
  tab: TabKey;
  slotByTab: Record<string, string>;
  filter: ItemFilter;
  filterMenuOpen: boolean;
  colorTarget: ColorTarget | null;
  /** Sheet snap: peek = collapsed to its handle, half = default, full = covers the avatar. */
  sheet: Snap;
  /** Selected emote slot, or null before the first tap. */
  emoteSlot: number | null;
  /** Emote tapped once and waiting for the EQUIP tap. */
  pendingEmote: string | null;
  /** True while "Equipped" replaces the name chip. */
  equippedToast: boolean;
};

export type BackpackAction =
  | { type: 'mode'; mode: Mode }
  | { type: 'tab'; tab: TabKey }
  | { type: 'slot'; slot: Slot }
  | { type: 'filter'; filter: ItemFilter }
  | { type: 'toggleFilterMenu' }
  | { type: 'closeFilterMenu' }
  | { type: 'openColor'; target: ColorTarget }
  | { type: 'closeColor' }
  | { type: 'sheet'; sheet: Snap }
  | { type: 'emoteSlot'; slot: number }
  | { type: 'emotePending'; id: string | null }
  | { type: 'emoteEquipped'; slot: number }
  | { type: 'toastDone' };

export const initialBackpack: BackpackState = {
  mode: 'wearables',
  tab: 'all',
  slotByTab: {},
  filter: 'all',
  filterMenuOpen: false,
  colorTarget: null,
  sheet: 'half',
  emoteSlot: null,
  pendingEmote: null,
  equippedToast: false,
};

export function reduceBackpack(
  state: BackpackState,
  action: BackpackAction,
): BackpackState {
  switch (action.type) {
    case 'mode':
      return {
        ...state,
        mode: action.mode,
        filterMenuOpen: false,
        colorTarget: null,
        pendingEmote: null,
        equippedToast: false,
      };
    case 'tab':
      return {
        ...state,
        tab: action.tab,
        filterMenuOpen: false,
        colorTarget: null,
      };
    case 'slot':
      return {
        ...state,
        slotByTab: { ...state.slotByTab, [state.tab]: action.slot.key },
        colorTarget: null,
      };
    case 'filter':
      return { ...state, filter: action.filter, filterMenuOpen: false };
    case 'toggleFilterMenu':
      return { ...state, filterMenuOpen: !state.filterMenuOpen };
    case 'closeFilterMenu':
      return { ...state, filterMenuOpen: false };
    case 'openColor':
      return { ...state, colorTarget: action.target };
    case 'closeColor':
      return { ...state, colorTarget: null };
    case 'sheet':
      return { ...state, sheet: action.sheet };
    case 'emoteSlot':
      return { ...state, emoteSlot: action.slot, pendingEmote: null };
    case 'emotePending':
      return { ...state, pendingEmote: action.id, equippedToast: false };
    case 'emoteEquipped':
      return {
        ...state,
        emoteSlot: action.slot,
        pendingEmote: null,
        equippedToast: true,
      };
    case 'toastDone':
      return { ...state, equippedToast: false };
  }
}

/** The slot selected under the current tab, defaulting to the tab's first slot. */
export function activeSlot(state: BackpackState): Slot | null {
  if (state.tab === 'all') {
    return null;
  }
  const slots = SLOTS[state.tab];
  return (
    slots.find(slot => slot.key === state.slotByTab[state.tab]) ?? slots[0]
  );
}
