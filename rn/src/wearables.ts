const BASE = 'urn:decentraland:off-chain:base-avatars:';

export type WearableItem = { id: string; label: string };
export type WearableCategory = {
  key: string;
  label: string;
  icon: string;
  items: WearableItem[];
};

const item = (id: string, label: string): WearableItem => ({
  id: BASE + id,
  label,
});

export const CATEGORIES: WearableCategory[] = [
  {
    key: 'hair',
    label: 'Hair',
    icon: '💇',
    items: [
      item('casual_hair_01', 'Casual'),
      item('hair_punk', 'Punk'),
      item('keanu_hair', 'Keanu'),
      item('cool_hair', 'Cool'),
      item('slicked_hair', 'Slicked'),
      item('curly_hair', 'Curly'),
    ],
  },
  {
    key: 'eyewear',
    label: 'Eyewear',
    icon: '🕶️',
    items: [
      item('matrix_sunglasses', 'Matrix'),
      item('retro_sunglasses', 'Retro'),
      item('heart_glasses', 'Heart'),
      item('black_sun_glasses', 'Black'),
    ],
  },
  {
    key: 'feet',
    label: 'Shoes',
    icon: '👟',
    items: [
      item('sneakers', 'Sneakers'),
      item('classic_shoes', 'Classic'),
      item('sport_black_shoes', 'Sport'),
      item('SchoolShoes', 'School'),
    ],
  },
];
