import type { common as en } from '../en/common';

export const common: Partial<Record<keyof typeof en, string>> = {
  BACKPACK_BACKPACK: 'Mochila',
  SETTINGS_SETTINGS: 'Ajustes',
  APP_SEARCH: 'Buscar',
  APP_CLOSE: 'Fechar',
  APP_TICKET: 'Ingressos',
  APP_CREDITS: 'Créditos',
};
