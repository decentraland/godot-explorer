/** What the world HUD hears from the core, shaped for its reducers. */

export type LoadingProgress = {
  loaded: number;
  total: number;
  mbps: number;
  /** 0..100 */
  percent: number;
};

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

/** `chat` with the sender already resolved by the engine; `profile` is absent for system lines. */
export type ChatEvent = {
  type: 'chat';
  data: {
    address: string;
    message: string;
    timestamp: number;
    profile?: ChatProfile;
  };
};

export type HudEvent =
  | ChatEvent
  | { type: 'loading_progress'; data: LoadingProgress }
  | { type: 'unread_chat'; data: { count: number } }
  | { type: 'open_backpack'; data: { emotes: boolean } };

/** A line in the chat model: `system` lines come from the engine with BBCode in `message`. */
export type ChatLine = {
  id: string;
  address: string;
  message: string;
  timestamp: number;
};
