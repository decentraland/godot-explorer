import { SLOTS, Slot, TabKey } from './taxonomy';
import { ItemFilter } from './selection';
import { ColorTarget } from './taxonomy';

export type Mode = 'wearables' | 'emotes';
export type SheetState = 'expanded' | 'collapsed';

export type BackpackState = {
  mode: Mode;
  tab: TabKey;
  slotByTab: Record<string, string>;
  filter: ItemFilter;
  filterMenuOpen: boolean;
  colorTarget: ColorTarget | null;
  sheet: SheetState;
  emoteSlot: number;
};

export type BackpackAction =
  | { type: 'mode'; mode: Mode }
  | { type: 'tab'; tab: TabKey }
  | { type: 'slot'; slot: Slot }
  | { type: 'filter'; filter: ItemFilter }
  | { type: 'toggleFilterMenu' }
  | { type: 'openColor'; target: ColorTarget }
  | { type: 'closeColor' }
  | { type: 'sheet'; sheet: SheetState }
  | { type: 'emoteSlot'; slot: number };

export const initialBackpack: BackpackState = {
  mode: 'wearables',
  tab: 'all',
  slotByTab: {},
  filter: 'all',
  filterMenuOpen: false,
  colorTarget: null,
  sheet: 'expanded',
  emoteSlot: 0,
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
        colorTarget: action.slot.category ? null : action.slot.color ?? null,
      };
    case 'filter':
      return { ...state, filter: action.filter, filterMenuOpen: false };
    case 'toggleFilterMenu':
      return { ...state, filterMenuOpen: !state.filterMenuOpen };
    case 'openColor':
      return { ...state, colorTarget: action.target };
    case 'closeColor':
      return { ...state, colorTarget: null };
    case 'sheet':
      return { ...state, sheet: action.sheet };
    case 'emoteSlot':
      return { ...state, emoteSlot: action.slot };
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
