import type { AvatarColorTarget } from '../GodotBridge';
import type { I18nKey } from '../i18n';

export type TabKey = 'all' | 'body' | 'face' | 'clothing' | 'extras';

/** What the editor titles say; hair, facial hair and eyebrows all edit the one `hair` colour. */
export type ColorTarget = 'skin' | 'hair' | 'facial_hair' | 'eyebrows' | 'eyes';

/** A row entry under a tab: a wearable category, optionally with the colour its swatches edit. */
export type Slot = {
  key: string;
  label: I18nKey;
  category?: string;
  color?: ColorTarget;
};

export const TABS: { key: TabKey; label: I18nKey }[] = [
  { key: 'all', label: 'BACKPACK_ALL_CAPS' },
  { key: 'body', label: 'BACKPACK_BODY' },
  { key: 'face', label: 'BACKPACK_FACE' },
  { key: 'clothing', label: 'BACKPACK_CLOTHING' },
  { key: 'extras', label: 'BACKPACK_EXTRAS' },
];

export const SLOTS: Record<Exclude<TabKey, 'all'>, Slot[]> = {
  body: [
    { key: 'shape', label: 'BACKPACK_SLOT_SHAPE', category: 'body_shape' },
    {
      key: 'skin',
      label: 'BACKPACK_SLOT_SKIN',
      category: 'skin',
      color: 'skin',
    },
  ],
  face: [
    {
      key: 'hair',
      label: 'BACKPACK_SLOT_HAIR',
      category: 'hair',
      color: 'hair',
    },
    {
      key: 'facial_hair',
      label: 'BACKPACK_SLOT_FACIAL_HAIR',
      category: 'facial_hair',
      color: 'facial_hair',
    },
    {
      key: 'eyes',
      label: 'BACKPACK_SLOT_EYE',
      category: 'eyes',
      color: 'eyes',
    },
    {
      key: 'eyebrows',
      label: 'BACKPACK_SLOT_EYEBROWS',
      category: 'eyebrows',
      color: 'eyebrows',
    },
    { key: 'mouth', label: 'BACKPACK_SLOT_MOUTH', category: 'mouth' },
  ],
  clothing: [
    { key: 'upper_body', label: 'BACKPACK_SLOT_CHEST', category: 'upper_body' },
    { key: 'lower_body', label: 'BACKPACK_SLOT_LEGS', category: 'lower_body' },
    { key: 'feet', label: 'BACKPACK_SLOT_FEET', category: 'feet' },
    { key: 'hands_wear', label: 'BACKPACK_SLOT_HAND', category: 'hands_wear' },
  ],
  extras: [
    { key: 'hat', label: 'BACKPACK_SLOT_HAT', category: 'hat' },
    { key: 'helmet', label: 'BACKPACK_SLOT_HELMETS', category: 'helmet' },
    { key: 'tiara', label: 'BACKPACK_SLOT_TIARAS', category: 'tiara' },
    { key: 'eyewear', label: 'BACKPACK_SLOT_GLASSES', category: 'eyewear' },
    { key: 'mask', label: 'BACKPACK_SLOT_MASK', category: 'mask' },
    { key: 'earring', label: 'BACKPACK_SLOT_EARRINGS', category: 'earring' },
  ],
};

/** Body parts the avatar can't be without; the client ignores unequipping them. */
const REQUIRED = new Set(['body_shape', 'eyes', 'eyebrows', 'mouth']);

export const isRemovable = (category: string) => !REQUIRED.has(category);

export const COLOR_TITLES: Record<ColorTarget, I18nKey> = {
  skin: 'BACKPACK_COLOR_SKIN',
  hair: 'BACKPACK_COLOR_HAIR',
  facial_hair: 'BACKPACK_COLOR_FACIAL_HAIR',
  eyebrows: 'BACKPACK_COLOR_EYEBROWS',
  eyes: 'BACKPACK_COLOR_EYES',
};

/** The engine only knows skin, hair and eyes; the other hair-like targets share `hair`. */
export const bridgeTarget = (target: ColorTarget): AvatarColorTarget =>
  target === 'skin' || target === 'eyes' ? target : 'hair';

/** Palettes from the lobby editor (godot/src/ui/components/avatar presets). */
export const PALETTES: Record<AvatarColorTarget, string[]> = {
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

export type CameraFocus = 'overall' | 'head';

/** Face slots zoom to the head (Figma backpack-a 122-130); everything else shows the whole avatar. */
export const cameraFocusFor = (tab: TabKey): CameraFocus =>
  tab === 'face' ? 'head' : 'overall';
