import type { backpack as en } from '../en/backpack';

export const backpack: Partial<Record<keyof typeof en, string>> = {
  BACKPACK_WEARABLES: 'Wearables',
  BACKPACK_EMOTES: 'Emotes',
  BACKPACK_ALL: 'Tudo',
  BACKPACK_ALL_CAPS: 'TUDO',
  BACKPACK_BODY: 'CORPO',
  BACKPACK_FACE: 'FACE',
  BACKPACK_CLOTHING: 'VESTUÁRIO',
  BACKPACK_EXTRAS: 'EXTRAS',
  BACKPACK_COLLECTIBLES: 'Colecionáveis',
  BACKPACK_EQUIPPED: 'Equipado',
  BACKPACK_NOTHING_HERE_YET: 'Ainda nada aqui.',
};
