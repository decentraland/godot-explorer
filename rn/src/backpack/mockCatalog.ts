import type { Rarity } from '../ds';
import type { AvatarState } from '../core/native';
import type { Catalog } from './api';
import { BASE_PREFIX, FEMALE, MALE, Wearable } from './catalog';
import { DEFAULT_EMOTES, Emote } from './emotes';

const COLLECTION = 'urn:decentraland:matic:collections-v2:0xmock:';

let counter = 0;

function wearable(
  category: string,
  name: string,
  rarity: Rarity = 'base',
): Wearable {
  const base = rarity === 'base';
  counter += 1;
  return {
    id: base ? `${BASE_PREFIX}${name}` : `${COLLECTION}${counter}`,
    name,
    category,
    thumbnail: '',
    rarity,
    bodyShapes: [MALE, FEMALE],
    collectible: !base,
  };
}

const group = (category: string, entries: [string, Rarity?][]): Wearable[] =>
  entries.map(([name, rarity]) => wearable(category, name, rarity));

const BODY_SHAPES: Wearable[] = [
  { ...wearable('body_shape', 'BaseFemale'), bodyShapes: [FEMALE] },
  { ...wearable('body_shape', 'BaseMale'), bodyShapes: [MALE] },
];

export const MOCK_WEARABLES: Wearable[] = [
  ...BODY_SHAPES,
  ...group('hair', [
    ['short_hair'],
    ['punk_hair'],
    ['long_hair'],
    ['pompadour'],
    ['bun'],
    ['curly_hair'],
  ]),
  ...group('facial_hair', [
    ['beard'],
    ['goatee'],
    ['mustache'],
    ['full_beard'],
    ['stubble'],
  ]),
  ...group('eyes', [['eyes_00'], ['eyes_01'], ['eyes_02'], ['eyes_03']]),
  ...group('eyebrows', [['brows_00'], ['brows_01'], ['brows_02']]),
  ...Array.from({ length: 12 }, (_, i) =>
    wearable('mouth', `mouth_${String(i).padStart(2, '0')}`),
  ),
  ...group('upper_body', [
    ['hoodie'],
    ['puffer_jacket'],
    ['skeleton_torso', 'epic'],
    ['yellow_tee', 'unique'],
    ['xmas_sweater', 'rare'],
    ['brown_hoodie', 'common'],
    ['green_hoodie'],
    ['green_hoodie_2'],
    ['green_hoodie_3'],
    ['green_hoodie_4'],
    ['green_hoodie_5'],
    ['green_hoodie_6'],
    ['green_hoodie_7'],
    ['green_hoodie_8'],
    ['green_hoodie_9'],
  ]),
  ...group('lower_body', [['pants'], ['jeans'], ['shorts'], ['skirt']]),
  ...group('feet', [['sneakers'], ['boots'], ['sandals'], ['heels']]),
  ...group('hands_wear', [
    ['watch', 'uncommon'],
    ['cube_bracelet', 'rare'],
    ['gloves'],
    ['mittens'],
  ]),
  ...group('hat', [
    ['fox_hat', 'legendary'],
    ['cap'],
    ['beanie'],
    ['sombrero', 'epic'],
    ['crown', 'mythic'],
    ['bucket_hat'],
  ]),
  ...group('tiara', [
    ['tiara_1', 'exotic'],
    ['tiara_2', 'epic'],
    ['tiara_3', 'rare'],
    ['tiara_4', 'common'],
    ['tiara_5', 'uncommon'],
  ]),
  ...group('eyewear', [
    ['spiral_goggles', 'rare'],
    ['flower_glasses', 'epic'],
    ['heart_glasses', 'uncommon'],
    ['aviators'],
    ['round_glasses'],
    ['visor', 'legendary'],
    ['monocle'],
    ['shades'],
    ['party_glasses', 'common'],
  ]),
  ...group('earring', [
    ['crystal_earrings', 'rare'],
    ['hoops'],
    ['studs'],
    ['pearls', 'epic'],
    ['feathers'],
    ['ear_cuff'],
  ]),
];

/** Skin collectibles; not part of MOCK_WEARABLES because the base catalog has none (empty state). */
export const MOCK_SKINS: Wearable[] = group('skin', [
  ['skeleton_skin', 'epic'],
  ['zombie_skin', 'rare'],
]);

const COLLECTIBLE_EMOTES: [string, Rarity][] = [
  ['Lime Spin', 'exotic'],
  ['Duo Dance', 'epic'],
  ['Blue Wave', 'rare'],
  ['Orange Pose', 'unique'],
];

export const MOCK_EMOTES: Emote[] = [
  ...DEFAULT_EMOTES.map((emote, index) => ({
    ...emote,
    name: `Emote Name${String(index + 1).padStart(2, '0')}`,
  })),
  ...COLLECTIBLE_EMOTES.map(([name, rarity], index) => ({
    id: `collectible-${index}`,
    urn: `${COLLECTION}emote-${index}`,
    name,
    rarity,
    collectible: true,
  })),
];

export const MOCK_CATALOG: Catalog = {
  wearables: MOCK_WEARABLES,
  emotes: MOCK_EMOTES,
};

const idOf = (name: string) =>
  MOCK_WEARABLES.find(item => item.name === name)?.id ?? '';

export const MOCK_AVATAR: AvatarState = {
  bodyShape: MALE,
  wearables: [
    'hoodie',
    'pants',
    'sneakers',
    'short_hair',
    'beard',
    'eyes_00',
    'brows_00',
    'mouth_00',
  ].map(idOf),
  emotes: DEFAULT_EMOTES.slice(0, 10).map(emote => emote.urn),
  skin: '#ffe4c6',
  hair: '#3b2412',
  eyes: '#4a3220',
};
