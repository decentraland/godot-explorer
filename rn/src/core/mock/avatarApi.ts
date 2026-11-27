import { ColorTarget } from '../native';
import type { CatalogItem } from '../native';
import type { AvatarApi } from '../api/avatar';
import { isRemovable } from './avatar';
import type { MockAvatar } from './avatar';
import type { Wearable } from '../../backpack/catalog';

const BASE = 'urn:decentraland:off-chain:base-avatars:';
const MALE = `${BASE}BaseMale`;
const FEMALE = `${BASE}BaseFemale`;

const PALETTES = {
  [ColorTarget.Skin]: ['#ffe4c6', '#d9a982', '#a56f4a', '#6b3f28'],
  [ColorTarget.Hair]: ['#1c1c1c', '#5a3410', '#8a2515', '#e0952c'],
  [ColorTarget.Eyes]: ['#2b2b2b', '#6b4a2e', '#2f6fb0', '#2f6f4a'],
};

const EMOTES: [string, string][] = [
  ['handsair', 'Hands Air'],
  ['wave', 'Wave'],
  ['fistpump', 'Fist Pump'],
  ['dance', 'Dance'],
  ['raiseHand', 'Raise Hand'],
  ['clap', 'Clap'],
  ['money', 'Money'],
  ['kiss', 'Kiss'],
  ['shrug', 'Shrug'],
  ['headexplode', 'Head Explode'],
  ['cry', 'Cry'],
  ['dab', 'Dab'],
  ['disco', 'Disco'],
  ['dontsee', "Don't See"],
  ['hammer', 'Hammer'],
  ['hohoho', 'Ho Ho Ho'],
  ['robot', 'Robot'],
  ['snowfall', 'Snowfall'],
  ['tektonik', 'Tektonik'],
  ['tik', 'Tik'],
  ['confettipopper', 'Confetti Popper'],
];

const item = (
  name: string,
  category: string,
  bodyShapes: string[],
  rarity = 'base',
): CatalogItem => ({
  urn: BASE + name,
  name,
  category,
  thumbnail: '',
  rarity,
  bodyShapes,
  collectible: rarity !== 'base',
});

/** A small base catalog for builds without the engine; the real one comes from the lambdas. Each slot has
 * several items so selection flows have something to choose between. */
const CATALOG: CatalogItem[] = [
  item('BaseMale', 'body_shape', [MALE]),
  item('BaseFemale', 'body_shape', [FEMALE]),
  item('eyes_00', 'eyes', [MALE, FEMALE]),
  item('eyes_01', 'eyes', [MALE, FEMALE]),
  item('eyes_02', 'eyes', [MALE, FEMALE]),
  item('eyebrows_00', 'eyebrows', [MALE, FEMALE]),
  item('eyebrows_01', 'eyebrows', [MALE, FEMALE]),
  item('mouth_00', 'mouth', [MALE, FEMALE]),
  item('mouth_01', 'mouth', [MALE, FEMALE]),
  item('short_hair', 'hair', [MALE, FEMALE]),
  item('long_hair', 'hair', [MALE, FEMALE]),
  item('mohawk', 'hair', [MALE, FEMALE]),
  item('goatee', 'facial_hair', [MALE, FEMALE]),
  item('full_beard', 'facial_hair', [MALE, FEMALE]),
  item('green_hoodie', 'upper_body', [MALE, FEMALE]),
  item('blue_tshirt', 'upper_body', [MALE, FEMALE]),
  item('red_jacket', 'upper_body', [MALE, FEMALE]),
  item('brown_pants', 'lower_body', [MALE, FEMALE]),
  item('black_shorts', 'lower_body', [MALE, FEMALE]),
  item('sneakers', 'feet', [MALE, FEMALE]),
  item('boots', 'feet', [MALE, FEMALE]),
  item('cap', 'hat', [MALE, FEMALE]),
  item('beanie', 'hat', [MALE, FEMALE]),
  item('sunglasses', 'eyewear', [MALE, FEMALE]),
  item('round_glasses', 'eyewear', [MALE, FEMALE]),
  item('laser_visor', 'eyewear', [MALE, FEMALE], 'legendary'),
  item('neon_jacket', 'upper_body', [MALE, FEMALE], 'rare'),
  item('leather_gloves', 'hands_wear', [MALE, FEMALE]),
  item('fingerless_gloves', 'hands_wear', [MALE, FEMALE]),
  item('bike_helmet', 'helmet', [MALE, FEMALE]),
  item('knight_helmet', 'helmet', [MALE, FEMALE]),
  item('silver_tiara', 'tiara', [MALE, FEMALE]),
  item('crystal_tiara', 'tiara', [MALE, FEMALE], 'epic'),
  item('hoop_earrings', 'earring', [MALE, FEMALE]),
  item('stud_earrings', 'earring', [MALE, FEMALE]),
];

/** The avatar data slice in memory; loading the catalog also teaches the mock avatar its categories. */
export function createMockAvatarApi(avatar: MockAvatar): AvatarApi {
  return {
    loadAvatarCatalog: async () => {
      avatar.setWearables(
        CATALOG.map(c => ({
          id: c.urn,
          name: c.name,
          category: c.category,
          thumbnail: c.thumbnail,
          rarity: c.rarity as Wearable['rarity'],
          bodyShapes: c.bodyShapes,
          collectible: c.collectible,
        })),
      );
      return CATALOG;
    },
    avatarPalette: target => PALETTES[target],
    baseEmotes: () =>
      EMOTES.map(([id, name]) => ({
        id,
        urn: `urn:decentraland:off-chain:base-emotes:${id}`,
        name,
      })),
    isRemovableCategory: isRemovable,
  };
}
