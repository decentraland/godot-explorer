// Stand-in for the Godot engine when the native GodotBridge module is not linked (simulators,
// emulators, Expo Go): replays the event stream a real session produces so the UI can be built
// without the 3D client.
import type { GodotCommand } from '../GodotBridge';
import type { GodotEvent } from '../events';

const STATE_INTERVAL_MS = 250;
const LOADING_STARTED_MS = 300;
const LOADING_FINISHED_MS = 1500;
const PARCEL_SIZE = 16;
const MOCK_ADDRESS = '0xm0ck00000000000000000000000000000000000';

type Listener = (event: GodotEvent) => void;

export type MockBridge = {
  sendCommand(command: GodotCommand): void;
  subscribe(listener: Listener): () => void;
};

export function createMockBridge(): MockBridge {
  const listeners = new Set<Listener>();
  const timers: ReturnType<typeof setTimeout>[] = [];
  let ticker: ReturnType<typeof setInterval> | null = null;
  let position: [number, number, number] = [8, 0, 8];
  let step = 0;

  const emit = (event: GodotEvent) => listeners.forEach(listener => listener(event));

  const emitState = () => {
    step += 1;
    position = [position[0] + Math.cos(step / 8) * 0.2, 0, position[2] + Math.sin(step / 8) * 0.2];
    emit({
      type: 'state',
      data: {
        parcel: [Math.floor(position[0] / PARCEL_SIZE), Math.floor(position[2] / PARCEL_SIZE)],
        realm: 'mock',
        fps: 60,
        position,
      },
    });
  };

  const start = () => {
    timers.push(setTimeout(() => emit({ type: 'ready', data: {} }), 0));
    timers.push(setTimeout(() => emit({ type: 'loading', data: { state: 'started' } }), LOADING_STARTED_MS));
    timers.push(setTimeout(() => emit({ type: 'loading', data: { state: 'finished' } }), LOADING_FINISHED_MS));
    ticker = setInterval(emitState, STATE_INTERVAL_MS);
  };

  const stop = () => {
    timers.splice(0).forEach(clearTimeout);
    if (ticker) {
      clearInterval(ticker);
      ticker = null;
    }
  };

  return {
    sendCommand(command) {
      switch (command.type) {
        case 'teleport':
          position = [command.x * PARCEL_SIZE + 8, 0, command.y * PARCEL_SIZE + 8];
          emitState();
          break;
        case 'send_chat':
          emit({
            type: 'chat',
            data: { address: MOCK_ADDRESS, message: command.text, timestamp: Date.now() },
          });
          break;
        case 'enter_world':
          emit({ type: 'world', data: { state: 'entered' } });
          break;
        case 'exit_world':
          emit({ type: 'world', data: { state: 'exited' } });
          emit({ type: 'ready', data: {} });
          break;
        default:
          break;
      }
    },
    subscribe(listener) {
      listeners.add(listener);
      if (listeners.size === 1) {
        start();
      }
      return () => {
        listeners.delete(listener);
        if (listeners.size === 0) {
          stop();
        }
      };
    },
  };
}
