import { NativeEventEmitter, NativeModules } from 'react-native';
import { GodotEvent, parseEvent } from './events';
import { createMockBridge } from './godot/mockBridge';

/** Menus are portrait-locked; a scene is landscape-locked. */
export type Orientation = 'portrait' | 'landscape';

type NativeBridge = {
  sendCommand(json: string): void;
  /** Boots the Godot engine in avatar-preview-only mode (no world). Idempotent. */
  startEngine(): void;
  setOrientation(mode: Orientation): void;
};

// Bridgeless RN reports a module that is not linked as null, the legacy bridge as undefined.
const native: NativeBridge | null = NativeModules.GodotBridge ?? null;

/** False when the app runs without the Godot engine (simulator, Expo Go): events are mocked. */
export const godotAvailable = native !== null;

const mock = godotAvailable ? null : createMockBridge();

export type AvatarColorTarget = 'skin' | 'hair' | 'eyes';

export type GodotCommand =
  | { type: 'teleport'; x: number; y: number }
  | { type: 'send_chat'; text: string }
  | { type: 'emote'; urn: string }
  | {
      type: 'backpack_open' | 'backpack_place';
      x: number;
      y: number;
      w: number;
      h: number;
    }
  | { type: 'backpack_close' }
  | { type: 'equip'; urn: string }
  | { type: 'unequip'; urn: string }
  | { type: 'set_body_shape'; urn: string }
  | { type: 'set_color'; target: AvatarColorTarget; color: string }
  | { type: 'play_emote'; urn: string }
  | { type: 'set_emote_slot'; slot: number; urn: string }
  | { type: 'randomize_avatar' }
  | { type: 'set_name'; name: string }
  | { type: 'enter_world'; x: number; y: number; realm: string }
  | { type: 'exit_world' }
  | { type: 'req'; id: string; method: string; params?: unknown }
  | { type: 'set_hud_insets'; top: number; right: number; bottom: number; left: number }
  | { type: 'set_chat_width'; width: number }
  | { type: 'chat_open'; open: boolean }
  | { type: 'hide_ui'; hidden: boolean }
  | { type: 'focus_camera'; target: CameraTarget };

/** Avatar preview framing: a wearable category, or the whole body. */
export type CameraTarget = 'overall' | 'head' | 'torso' | 'legs' | 'feet' | 'hands';

export function sendCommand(command: GodotCommand): void {
  if (mock) {
    mock.sendCommand(command);
    return;
  }
  native?.sendCommand(JSON.stringify(command));
}

export function startEngine(): void {
  native?.startEngine();
}

export function setOrientation(mode: Orientation): void {
  native?.setOrientation(mode);
}

export function subscribe(listener: (event: GodotEvent) => void): () => void {
  if (mock) {
    return mock.subscribe(listener);
  }
  const subscription = new NativeEventEmitter(
    NativeModules.GodotBridge,
  ).addListener('GodotEvent', (json: unknown) => {
    const event = typeof json === 'string' ? parseEvent(json) : null;
    if (event) {
      listener(event);
    }
  });
  return () => subscription.remove();
}
