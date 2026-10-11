// VIBE-CODED SPIKE: bridge event types and the pure reducer that turns them into HUD state.

export type GodotEvent =
  | { type: 'ready'; data: Record<string, never> }
  | { type: 'loading'; data: { state: 'started' | 'finished' } }
  | {
      type: 'state';
      data: { parcel: [number, number]; realm: string; fps: number; position: [number, number, number] };
    }
  | { type: 'chat'; data: { address: string; message: string; timestamp: number } };

export type ChatLine = { address: string; message: string; timestamp: number };

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

export function parseEvent(json: string): GodotEvent | null {
  try {
    const parsed: unknown = JSON.parse(json);
    if (typeof parsed !== 'object' || parsed === null || !('type' in parsed) || !('data' in parsed)) {
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
      return { ...state, chat: [...state.chat, event.data].slice(-MAX_CHAT_LINES) };
    default:
      return state;
  }
}
