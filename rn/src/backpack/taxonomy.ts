export type TabKey = 'all' | 'body' | 'face' | 'clothing' | 'extras';
export type ColorTarget = 'skin' | 'hair' | 'eyes';

/** A row entry under a tab: a wearable category, a color-only slot (skin), or both. */
export type Slot = {
  key: string;
  label: string;
  category?: string;
  color?: ColorTarget;
};

export const TABS: { key: TabKey; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'body', label: 'Body' },
  { key: 'face', label: 'Face' },
  { key: 'clothing', label: 'Clothing' },
  { key: 'extras', label: 'Extras' },
];

export const SLOTS: Record<Exclude<TabKey, 'all'>, Slot[]> = {
  body: [
    { key: 'shape', label: 'Shape', category: 'body_shape' },
    { key: 'skin', label: 'Skin', color: 'skin' },
  ],
  face: [
    { key: 'hair', label: 'Hair', category: 'hair', color: 'hair' },
    {
      key: 'facial_hair',
      label: 'Facial Hair',
      category: 'facial_hair',
      color: 'hair',
    },
    { key: 'eyes', label: 'Eye', category: 'eyes', color: 'eyes' },
    { key: 'eyebrows', label: 'Eyebrows', category: 'eyebrows', color: 'hair' },
    { key: 'mouth', label: 'Mouth', category: 'mouth' },
  ],
  clothing: [
    { key: 'upper_body', label: 'Chest', category: 'upper_body' },
    { key: 'lower_body', label: 'Legs', category: 'lower_body' },
    { key: 'feet', label: 'Feet', category: 'feet' },
    { key: 'hands_wear', label: 'Hands', category: 'hands_wear' },
  ],
  extras: [
    { key: 'hat', label: 'Hat', category: 'hat' },
    { key: 'helmet', label: 'Helmets', category: 'helmet' },
    { key: 'tiara', label: 'Tiaras', category: 'tiara' },
    { key: 'eyewear', label: 'Glasses', category: 'eyewear' },
    { key: 'mask', label: 'Mask', category: 'mask' },
    { key: 'earring', label: 'Earrings', category: 'earring' },
  ],
};

/** Body parts the avatar can't be without; the client ignores unequipping them. */
const REQUIRED = new Set(['body_shape', 'eyes', 'eyebrows', 'mouth']);

export const isRemovable = (category: string) => !REQUIRED.has(category);

export const COLOR_TITLES: Record<ColorTarget, string> = {
  skin: 'Skin color',
  hair: 'Hair color',
  eyes: 'Eye color',
};

/** Palettes from the lobby editor (godot/src/ui/components/avatar presets). */
export const PALETTES: Record<ColorTarget, string[]> = {
  skin: [
    '#ffe4c6',
    '#ffdcb6',
    '#f3c9a8',
    '#d9a982',
    '#c49168',
    '#a56f4a',
    '#8a5a3a',
    '#6b3f28',
    '#4a2a1a',
  ],
  hair: [
    '#1c1c1c',
    '#3b2412',
    '#5a3410',
    '#7b4c14',
    '#9a6338',
    '#8a2515',
    '#e0952c',
    '#c9b27a',
    '#d9d9d9',
  ],
  eyes: [
    '#2b2b2b',
    '#4a3220',
    '#6b4a2e',
    '#2f6f4a',
    '#2f6fb0',
    '#6a8fa8',
    '#7a3f8f',
    '#9a9a9a',
  ],
};
