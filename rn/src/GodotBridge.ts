import { NativeEventEmitter, NativeModules } from 'react-native';
import { GodotEvent, parseEvent } from './events';

export type Orientation = 'portrait' | 'sensor';

type NativeBridge = {
  sendCommand(json: string): void;
  /** Boots the Godot engine in avatar-preview-only mode (no world). Idempotent. */
  startEngine(): void;
  setOrientation(mode: Orientation): void;
};

const native: NativeBridge | undefined = NativeModules.GodotBridge;

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
  | { type: 'exit_world' };

export function sendCommand(command: GodotCommand): void {
  native?.sendCommand(JSON.stringify(command));
}

export function startEngine(): void {
  native?.startEngine();
}

export function setOrientation(mode: Orientation): void {
  native?.setOrientation(mode);
}

export function subscribe(listener: (event: GodotEvent) => void): () => void {
  if (!native) {
    return () => {};
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
