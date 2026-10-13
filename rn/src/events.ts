// VIBE-CODED SPIKE: bridge event types and the pure reducer that turns them into HUD state.

export type GodotEvent =
  | { type: 'ready'; data: Record<string, never> }
  | { type: 'loading'; data: { state: 'started' | 'finished' } }
  | {
      type: 'state';
      data: {
        parcel: [number, number];
        realm: string;
        fps: number;
        position: [number, number, number];
      };
    }
  | {
      type: 'chat';
      data: ChatLine;
    }
  | { type: 'res'; data: ResponseData }
  | { type: 'loading_progress'; data: LoadingProgress }
  | { type: 'hud_state'; data: { hidden: boolean; chat_open: boolean } }
  | { type: 'unread_chat'; data: { count: number } }
  | { type: 'open_backpack'; data: { emotes: boolean } }
  | {
      type: 'settings_changed';
      data: { key: string; value: string | number | boolean };
    }
  | { type: 'avatar'; data: AvatarState }
  | { type: 'world'; data: { state: 'entered' | 'exited' } };

/** Equipped avatar as the Godot side sees it; colors are `#rrggbb`, empty emote slots are ''. */
export type AvatarState = {
  body_shape: string;
  wearables: string[];
  emotes: string[];
  skin: string;
  hair: string;
  eyes: string;
};

/** Sender profile Godot resolved for a chat line; absent for system lines and unknown senders. */
export type ChatProfile = {
  name: string;
  has_claimed_name: boolean;
  profile_picture_url: string;
};

export type ChatLine = {
  address: string;
  message: string;
  timestamp: number;
  profile?: ChatProfile;
};

/** Answer to a `req` command; `result` on success, `error` otherwise. */
export type ResponseData = { id: string; ok: boolean; result?: unknown; error?: string };

export type LoadingProgress = {
  loaded: number;
  total: number;
  mbps: number;
  percent: number;
};

export type Phase = 'booting' | 'loading' | 'playing';

export type HudState = {
  phase: Phase;
  parcel: [number, number] | null;
  realm: string;
  fps: number;
  position: [number, number, number] | null;
  chat: ChatLine[];
};

export const MAX_CHAT_LINES = 50;

export const initialHud: HudState = {
  phase: 'booting',
  parcel: null,
  realm: '',
  fps: 0,
  position: null,
  chat: [],
};

function isResponse(data: unknown): boolean {
  return (
    typeof data === 'object' &&
    data !== null &&
    typeof (data as { id?: unknown }).id === 'string'
  );
}

export function parseEvent(json: string): GodotEvent | null {
  try {
    const parsed: unknown = JSON.parse(json);
    if (
      typeof parsed !== 'object' ||
      parsed === null ||
      !('type' in parsed) ||
      !('data' in parsed)
    ) {
      return null;
    }
    const event = parsed as { type: unknown; data: unknown };
    if (event.type === 'res' && !isResponse(event.data)) {
      return null;
    }
    return parsed as GodotEvent;
  } catch {
    return null;
  }
}

export function reduceHud(state: HudState, event: GodotEvent): HudState {
  switch (event.type) {
    case 'state':
      return {
        ...state,
        phase: state.phase === 'booting' ? 'loading' : state.phase,
        parcel: event.data.parcel,
        realm: event.data.realm,
        fps: event.data.fps,
        position: event.data.position,
      };
    case 'loading':
      if (event.data.state === 'finished' && state.phase !== 'booting') {
        return { ...state, phase: 'playing' };
      }
      return state;
    case 'chat':
      return {
        ...state,
        chat: [...state.chat, event.data].slice(-MAX_CHAT_LINES),
      };
    default:
      return state;
  }
}
