import type { engine as en } from '../en/engine';

export const engine: Partial<Record<keyof typeof en, string>> = {
  EMOTES_EDIT: 'Editar',
  EMOTES_EMOTES: 'Emotes',
  EMOTES_EQUIP_EMOTE: 'Equipar Emote',
  TOOLTIP_ACTION_TAP: 'Tocar',
  TOOLTIP_ACTION_JUMP: 'Saltar',
  TOOLTIP_ACTION_PRIMARY: 'Principal',
  TOOLTIP_ACTION_SECONDARY: 'Secundaria',
  TOOLTIP_ACTION_1: 'Acción 1',
  TOOLTIP_ACTION_2: 'Acción 2',
  TOOLTIP_ACTION_3: 'Acción 3',
  TOOLTIP_ACTION_4: 'Acción 4',
  CHAT_SYSTEM_CHANGING_REALM:
    '[color=#ccc]Intentando cambiar al realm {realm}[/color]',
  CHAT_SYSTEM_CHANGING_WORLD:
    '[color=#ccc]Intentando cambiar al world {world}[/color]',
  CHAT_SYSTEM_NO_CAMERA: '[color=#ccc]🔴 No hay ninguna cámara activa[/color]',
  CHAT_SYSTEM_TELEPORTED:
    '[color=#ccc]🟢 Teletransportado a {location}[/color]',
  CHAT_SYSTEM_UNKNOWN_COMMAND: '[color=#ccc]🔴 Comando desconocido[/color]',
  TOOLTIP_VIEW_PROFILE: 'Ver perfil',
};
