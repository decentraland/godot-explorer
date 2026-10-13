/** Bridge events and commands the world HUD needs beyond the ones in `events.ts` / `GodotBridge.ts`. */

export type LoadingProgress = {
  loaded: number;
  total: number;
  mbps: number;
  /** 0..100 */
  percent: number;
};

/** `loading_progress` follows `loading{started}`; `failed` is `realm_change_failed`. */
export type LoadingEvent =
  | { type: 'loading'; data: { state: 'started' | 'finished' | 'failed' } }
  | { type: 'loading_progress'; data: LoadingProgress };

export type Bounds = [x: number, y: number, width: number, height: number];

export type ChatProfile = {
  address: string;
  /** Display name without the `#tag`. */
  name: string;
  /** `#1l0u` for unclaimed names, empty for claimed ones. */
  tag: string;
  claimed: boolean;
  /** `#rrggbb` nickname colour. */
  color: string;
  own: boolean;
  /** Face snapshot url, when the profile has one. */
  face?: string;
};

/** `chat` with the sender already resolved by Godot; `profile` is absent until wave 3 wires it. */
export type ChatEvent = {
  type: 'chat';
  data: {
    address: string;
    message: string;
    timestamp: number;
    profile?: ChatProfile;
  };
};

export type HudBridgeEvent =
  | LoadingEvent
  | ChatEvent
  | {
      type: 'hud_state';
      data: { joypadVisible: boolean; joypadBounds: Bounds | null };
    }
  | { type: 'unread_chat'; data: { count: number } }
  | { type: 'open_backpack'; data: { emotes: boolean } };

export type HudCommand =
  | {
      type: 'set_hud_insets';
      left: number;
      top: number;
      right: number;
      bottom: number;
    }
  | { type: 'set_chat_width'; px: number }
  | { type: 'chat_open'; open: boolean }
  | { type: 'hide_ui'; hidden: boolean }
  | { type: 'send_chat'; text: string }
  | { type: 'emote'; urn: string }
  | { type: 'teleport'; x: number; y: number }
  | { type: 'enter_world'; x: number; y: number; realm: string };

/** A line in the chat model: `system` lines come from Godot with BBCode in `message`. */
export type ChatLine = {
  id: string;
  address: string;
  message: string;
  timestamp: number;
};
