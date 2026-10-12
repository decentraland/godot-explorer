import { NativeEventEmitter, NativeModules } from 'react-native';
import { GodotEvent, parseEvent } from './events';

type NativeBridge = { sendCommand(json: string): void };

const native: NativeBridge | undefined = NativeModules.GodotBridge;

export type GodotCommand =
  | { type: 'teleport'; x: number; y: number }
  | { type: 'send_chat'; text: string }
  | { type: 'emote'; urn: string }
  | { type: 'backpack_open' | 'backpack_place'; x: number; y: number; w: number; h: number }
  | { type: 'backpack_close' }
  | { type: 'equip'; urn: string };

export function sendCommand(command: GodotCommand): void {
  native?.sendCommand(JSON.stringify(command));
}

export function subscribe(listener: (event: GodotEvent) => void): () => void {
  if (!native) {
    return () => {};
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
