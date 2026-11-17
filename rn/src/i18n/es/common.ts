import type { common as en } from '../en/common';

export const common: Partial<Record<keyof typeof en, string>> = {
  BACKPACK_BACKPACK: 'Backpack',
  SETTINGS_SETTINGS: 'Ajustes',
  APP_SEARCH: 'Buscar',
  APP_CLOSE: 'Cerrar',
  APP_TICKET: 'Entradas',
  APP_CREDITS: 'Créditos',
  APP_JUMP_IN: 'Entrar',
  APP_REMOVE: 'Quitar',
  APP_SEE_LESS: 'Ver menos',
  APP_SEE_MORE: 'Ver más',
};
