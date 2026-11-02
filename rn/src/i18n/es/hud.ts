import type { hud as en } from '../en/hud';

export const hud: Partial<Record<keyof typeof en, string>> = {
  LOADING_ENJOY_EXPERIENCE: 'Disfruta de la experiencia',
  LOADING_PERCENT: '{value} %',
  TIP_EMOTES:
    'Usa Emotes para saludar o reaccionar. Toca el icono para abrir la rueda de Emotes.',
  TIP_CREATOR_HUB:
    'Usa el Creator Hub para crear tus propios mundos y experiencias.',
  TIP_WEARABLES:
    'Los Wearables definen tu aspecto con el tiempo. Creados por la comunidad.',
  MODAL_SCENE_TIMEOUT_TITLE: 'La carga está tardando más de lo previsto',
  MODAL_SCENE_TIMEOUT_BODY:
    'Recargar la experiencia o entrar ahora ? La descarga seguirá en segundo plano.',
  MODAL_SCENE_TIMEOUT_PRIMARY: 'RECARGAR',
  MODAL_SCENE_TIMEOUT_SECONDARY: 'ENTRAR IGUALMENTE',
  COMMON_UNKNOWN_USER: 'Desconocido',
  CHAT_SYSTEM_SENDER: 'Sistema',
  CHAT_SYSTEM_WELCOME:
    '[color=#cfc][b]¡Te damos la bienvenida a Decentraland! Respeta a los demás y diviértete.[/b][/color]',
  CHAT_HEADER_NEARBY: 'Cerca',
  CHAT_WRITE_A_MESSAGE: 'Escribe un mensaje',
  CHAT_WRITE_A_MESSAGE_ELLIPSIS: 'Escribe un mensaje...',
  CHAT_SEND: 'ENVIAR',
  CONTROLS_FTUE_CHAT: 'Chat',
  CONTROLS_FTUE_EMOTES: 'Emotes',
  CONTROLS_FTUE_MENU: 'Menú',
  CONTROLS_FTUE_MOVEMENT: 'Movimiento del personaje',
  CONTROLS_FTUE_TAP_TO_START: 'Toca la pantalla para empezar a explorar',
};
