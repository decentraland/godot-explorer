import type { hud as en } from '../en/hud';

export const hud: Partial<Record<keyof typeof en, string>> = {
  LOADING_ENJOY_EXPERIENCE: 'Curta sua experiência',
  LOADING_PERCENT: '{value}%',
  TIP_EMOTES:
    'Use Emotes para acenar, reagir e celebrar. Toque no ícone de Emote para abrir a roda',
  TIP_CREATOR_HUB:
    'Use o Creator Hub e crie seus próprios locais e experiências.',
  TIP_WEARABLES:
    'Wearables feitos pela comunidade definem seu visual com o tempo.',
  MODAL_SCENE_TIMEOUT_TITLE: 'Carregamento demora mais que o esperado',
  MODAL_SCENE_TIMEOUT_BODY:
    'Recarregue a experiência ou entre agora. Ela deve continuar a carregar em segundo plano.',
  MODAL_SCENE_TIMEOUT_PRIMARY: 'RECARREGAR',
  MODAL_SCENE_TIMEOUT_SECONDARY: 'ENTRAR AGORA',
  COMMON_UNKNOWN_USER: 'Desconhecido',
  CHAT_SYSTEM_SENDER: 'Sistema',
  CHAT_SYSTEM_WELCOME:
    '[color=#cfc][b]Bem-vindo à Decentraland! Respeite todos e aproveite.[/b][/color]',
  CHAT_HEADER_NEARBY: 'Perto',
  CHAT_WRITE_A_MESSAGE: 'Digite mensagem',
  CHAT_WRITE_A_MESSAGE_ELLIPSIS: 'Digite mensagem...',
  CHAT_SEND: 'ENVIAR',
  CONTROLS_FTUE_CHAT: 'Chat',
  CONTROLS_FTUE_EMOTES: 'Emotes',
  CONTROLS_FTUE_MENU: 'Menu',
  CONTROLS_FTUE_MOVEMENT: 'Movimento do personagem',
  CONTROLS_FTUE_TAP_TO_START: 'Toque na tela para começar a explorar',
};
