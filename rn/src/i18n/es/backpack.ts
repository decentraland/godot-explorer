import type { backpack as en } from '../en/backpack';

export const backpack: Partial<Record<keyof typeof en, string>> = {
  BACKPACK_WEARABLES: 'Wearables',
  BACKPACK_EMOTES: 'Emotes',
  BACKPACK_ALL: 'Todo',
  BACKPACK_ALL_CAPS: 'TODO',
  BACKPACK_BODY: 'CUERPO',
  BACKPACK_FACE: 'CARA',
  BACKPACK_CLOTHING: 'ROPA',
  BACKPACK_EXTRAS: 'EXTRAS',
  BACKPACK_COLLECTIBLES: 'Coleccionables',
  BACKPACK_EQUIPPED: 'Equipado',
  BACKPACK_NOTHING_HERE_YET: 'Aquí no hay nada todavía',
};
