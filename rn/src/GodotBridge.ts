import { NativeEventEmitter, NativeModules } from 'react-native';
import { GodotEvent, parseEvent } from './events';
import { createMockBridge } from './godot/mockBridge';

type NativeBridge = { sendCommand(json: string): void };

// Bridgeless RN reports a module that is not linked as null, the legacy bridge as undefined.
const native: NativeBridge | null = NativeModules.GodotBridge ?? null;

/** False when the app runs without the Godot engine (simulator, Expo Go): events are mocked. */
export const godotAvailable = native !== null;

const mock = godotAvailable ? null : createMockBridge();

export type GodotCommand =
  | { type: 'teleport'; x: number; y: number }
  | { type: 'send_chat'; text: string }
  | { type: 'emote'; urn: string }
  | { type: 'backpack_open' | 'backpack_place'; x: number; y: number; w: number; h: number }
  | { type: 'backpack_close' }
  | { type: 'equip'; urn: string };

export function sendCommand(command: GodotCommand): void {
  if (mock) {
    mock.sendCommand(command);
    return;
  }
  native?.sendCommand(JSON.stringify(command));
}

export function subscribe(listener: (event: GodotEvent) => void): () => void {
  if (mock) {
    return mock.subscribe(listener);
  }
  const subscription = new NativeEventEmitter(NativeModules.GodotBridge).addListener(
    'GodotEvent',
    (json: unknown) => {
      const event = typeof json === 'string' ? parseEvent(json) : null;
      if (event) {
        listener(event);
      }
    },
  );
  return () => subscription.remove();
}
