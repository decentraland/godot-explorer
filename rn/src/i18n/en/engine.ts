/** Text the Godot engine draws itself (emote wheel, interaction pills, chat system lines). `scripts/i18n-engine.mjs` exports it to godot/locale. */
export const engine = {
  EMOTES_EDIT: 'Edit',
  EMOTES_EMOTES: 'Emotes',
  EMOTES_EQUIP_EMOTE: 'Equip Emote',
  TOOLTIP_ACTION_TAP: 'Tap',
  TOOLTIP_ACTION_JUMP: 'Jump',
  TOOLTIP_ACTION_PRIMARY: 'Primary',
  TOOLTIP_ACTION_SECONDARY: 'Secondary',
  TOOLTIP_ACTION_1: 'Action 1',
  TOOLTIP_ACTION_2: 'Action 2',
  TOOLTIP_ACTION_3: 'Action 3',
  TOOLTIP_ACTION_4: 'Action 4',
  CHAT_SYSTEM_CHANGING_REALM:
    '[color=#ccc]Trying to change to realm {realm}[/color]',
  CHAT_SYSTEM_CHANGING_WORLD:
    '[color=#ccc]Trying to change to world {world}[/color]',
  CHAT_SYSTEM_NO_CAMERA: '[color=#ccc]🔴 No active camera[/color]',
  CHAT_SYSTEM_TELEPORTED: '[color=#ccc]🟢 Teleported to {location}[/color]',
  CHAT_SYSTEM_UNKNOWN_COMMAND: '[color=#ccc]🔴 Unknown command[/color]',
  TOOLTIP_VIEW_PROFILE: 'View profile',
} as const;
