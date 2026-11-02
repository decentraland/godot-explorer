/** World HUD and loading cover. Keys mirror godot/locale/en.csv where a legacy key exists. */
export const hud = {
  LOADING_ENJOY_EXPERIENCE: 'Enjoy your Experience',
  LOADING_BY: 'By',
  LOADING_PROGRESS: '{loaded}/{total} resources at {speed} mb/s',
  LOADING_PERCENT: '{value}%',
  LOADING_CLOSE: 'Cancel loading',
  LOADING_TIP_LABEL: 'Tip:',
  TIP_EMOTES:
    'Use Emotes to wave, react, or celebrate. Tap the Emote icon to open the Emote Wheel.',
  LOADING_TIP_FRIENDS:
    'Add friends so you can find each other again. Open profile and tap "Add Friend".',
  LOADING_TIP_CHAT:
    'Open the chat and start a conversation with people nearby.',
  TIP_CARD_EXPLORE:
    'Explore thousands of scenes built by creators from around the world.',
  TIP_CREATOR_HUB:
    'Use the Creator Hub to build your own places and experiences.',
  TIP_WEARABLES:
    'Wearables shape how you appear over time. Made by the community.',
  MODAL_SCENE_TIMEOUT_TITLE: 'Loading is taking longer than expected',
  MODAL_SCENE_TIMEOUT_BODY:
    'You can reload the experience, or jump in now, it should keep loading in the background.',
  MODAL_SCENE_TIMEOUT_PRIMARY: 'RELOAD',
  MODAL_SCENE_TIMEOUT_SECONDARY: 'START ANYWAY',
  LOADING_FAILED_TITLE: "We couldn't load this place",
  LOADING_FAILED_BODY: 'Check your connection and try again.',
  LOADING_FAILED_BACK: 'BACK',
  LOADING_FAILED_RETRY: 'TRY AGAIN',
  HUD_MENU: 'Menu',
  HUD_CHAT: 'Chat',
  HUD_SHOW_UI: 'Show interface',
  COMMON_UNKNOWN_USER: 'Unknown user',
  CHAT_SYSTEM_SENDER: 'System',
  CHAT_SYSTEM_WELCOME:
    '[color=#cfc][b]Welcome to Decentraland! Respect others and have fun.[/b][/color]',
  CHAT_HEADER_NEARBY: 'Nearby',
  CHAT_WRITE_A_MESSAGE: 'Write a message',
  CHAT_WRITE_A_MESSAGE_ELLIPSIS: 'Write a message...',
  CHAT_SEND: 'SEND',
  CHAT_NEW_MESSAGES: 'New messages',
  CHAT_CLOSE: 'Close chat',
  CHAT_CLOSE_INPUT: 'Close keyboard',
  CHAT_LINK_OPEN_TITLE: 'Open external link?',
  CHAT_LINK_OPEN_BODY: 'You are about to leave Decentraland and open {url}.',
  CHAT_LINK_OPEN: 'OPEN',
  CHAT_LINK_CANCEL: 'CANCEL',
  CHAT_TELEPORT_TITLE: 'Teleport to {location}?',
  CHAT_TELEPORT_BODY: 'You will leave your current place.',
  CHAT_TELEPORT_GO: 'JUMP IN',
  CONTROLS_FTUE_CHAT: 'Chat',
  CONTROLS_FTUE_EMOTES: 'Emotes',
  CONTROLS_FTUE_MENU: 'Menu',
  CONTROLS_FTUE_MOVEMENT: 'Character movement',
  CONTROLS_FTUE_TAP_TO_START: 'Tap the screen to start exploring',
  WORLD_LEAVE_TITLE: 'Leave this world?',
  WORLD_LEAVE_BODY: 'You will go back to the menus.',
  WORLD_LEAVE_CONFIRM: 'LEAVE',
} as const;
