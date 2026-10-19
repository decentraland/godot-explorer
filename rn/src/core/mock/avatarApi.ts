import { ColorTarget } from '../native';
import type { CatalogItem } from '../native';
import type { AvatarApi } from '../api/avatar';
import { isRemovable } from './avatar';
import type { MockAvatar } from './avatar';

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
): CatalogItem => ({
  urn: BASE + name,
  name,
  category,
  thumbnail: '',
  rarity: 'base',
  bodyShapes,
  collectible: false,
});

/** A small base catalog for builds without the engine; the real one comes from the lambdas. */
const CATALOG: CatalogItem[] = [
  item('BaseMale', 'body_shape', [MALE]),
  item('BaseFemale', 'body_shape', [FEMALE]),
  item('eyes_00', 'eyes', [MALE]),
  item('f_eyes_00', 'eyes', [FEMALE]),
  item('eyebrows_00', 'eyebrows', [MALE]),
  item('f_eyebrows_00', 'eyebrows', [FEMALE]),
  item('mouth_00', 'mouth', [MALE]),
  item('f_mouth_00', 'mouth', [FEMALE]),
  item('short_hair', 'hair', [MALE, FEMALE]),
  item('green_hoodie', 'upper_body', [MALE, FEMALE]),
  item('brown_pants', 'lower_body', [MALE, FEMALE]),
  item('sneakers', 'feet', [MALE, FEMALE]),
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
          rarity: 'base',
          bodyShapes: c.bodyShapes,
          collectible: false,
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
