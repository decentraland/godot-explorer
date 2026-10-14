import { State } from '../native';
import type { ChatLine, Destination, Parcel } from '../native';
import type { MockBus } from './emitter';

const PROGRESS_STEPS = 8;
const PROGRESS_STEP_MS = 150;
const TOTAL_RESOURCES = 584;
const TICK_MS = 250;
const CHAT_START_MS = 1600;
const CHAT_EVERY_MS = 2500;
const PARCEL_SIZE = 16;
const MOCK_REALM = 'mock';

export const MOCK_OWN_ADDRESS = '0xm0ck00000000000000000000000000000000000';
export const MOCK_CLARA = '0xc1a000000000000000000000000000000001f0a';
export const MOCK_BOB = '0xb0b000000000000000000000000000000002c3d2';

const SCRIPT: Array<{ address: string; name: string; message: string }> = [
  { address: MOCK_CLARA, name: 'Clara', message: 'Teleported' },
  {
    address: MOCK_BOB,
    name: 'Bob',
    message: 'hey! anyone going to the theatre at 12,45?',
  },
  {
    address: MOCK_CLARA,
    name: 'Clara',
    message: 'check https://decentraland.org/events',
  },
  {
    address: MOCK_BOB,
    name: 'Bob',
    message: '@Mateo welcome to Genesis Plaza',
  },
];

const parcelAt = (x: number, z: number): Parcel => ({
  x: Math.floor(x / PARCEL_SIZE),
  y: Math.floor(z / PARCEL_SIZE),
});

export type MockWorld = {
  enter(destination: Destination): void;
  leave(): void;
  teleport(parcel: Parcel): void;
  say(text: string): void;
  setChatOpen(open: boolean): void;
};

/** A world that loads in a second, reports the player a few times a second and chats a little. */
export function createMockWorld(bus: MockBus): MockWorld {
  let timers: ReturnType<typeof setTimeout>[] = [];
  let ticker: ReturnType<typeof setInterval> | undefined;
  let position = { x: 8, y: 0, z: 8 };
  let realm = MOCK_REALM;
  let step = 0;
  let unread = 0;
  let chatOpen = false;

  const at = (ms: number, run: () => void) => {
    timers = [...timers, setTimeout(run, ms)];
  };

  const sendHud = () => {
    const snapshot = {
      parcel: parcelAt(position.x, position.z),
      realm,
      fps: 60,
      position,
    };
    bus.emit(l => l.hud(snapshot));
  };

  const tick = () => {
    step += 1;
    position = {
      x: position.x + Math.cos(step / 8) * 0.2,
      y: 0,
      z: position.z + Math.sin(step / 8) * 0.2,
    };
    sendHud();
  };

  const receive = (line: ChatLine, own: boolean) => {
    bus.emit(l => l.chat(line));
    if (!own && !chatOpen) {
      unread += 1;
      bus.emit(l => l.unreadChat(unread));
    }
  };

  const stop = () => {
    timers.forEach(clearTimeout);
    timers = [];
    if (ticker) {
      clearInterval(ticker);
      ticker = undefined;
    }
  };

  const script = () =>
    SCRIPT.forEach((entry, index) =>
      at(CHAT_START_MS + index * CHAT_EVERY_MS, () =>
        receive(
          {
            address: entry.address,
            message: entry.message,
            timestamp: Date.now(),
            profile: {
              name: entry.name,
              hasClaimedName: false,
              profilePictureUrl: '',
            },
          },
          false,
        ),
      ),
    );

  return {
    enter: destination => {
      stop();
      const parcel = destination.parcel ?? { x: 0, y: 0 };
      realm = destination.realm ?? MOCK_REALM;
      position = {
        x: parcel.x * PARCEL_SIZE + 8,
        y: 0,
        z: parcel.y * PARCEL_SIZE + 8,
      };
      step = 0;
      unread = 0;
      bus.setState(new State.Loading({ destination, progress: 0 }));
      for (let n = 1; n <= PROGRESS_STEPS; n += 1) {
        at(n * PROGRESS_STEP_MS, () => {
          const progress = n / PROGRESS_STEPS;
          bus.emit(l =>
            l.loadingDetail({
              loaded: BigInt(Math.round(TOTAL_RESOURCES * progress)),
              total: BigInt(TOTAL_RESOURCES),
              mbps: 2.4,
              percent: Math.round(100 * progress),
            }),
          );
          bus.setState(new State.Loading({ destination, progress }));
        });
      }
      at(PROGRESS_STEPS * PROGRESS_STEP_MS, () => {
        bus.setState(new State.InWorld({ realm, parcel }));
        ticker = setInterval(tick, TICK_MS);
      });
      script();
    },
    leave: stop,
    teleport: parcel => {
      position = {
        x: parcel.x * PARCEL_SIZE + 8,
        y: 0,
        z: parcel.y * PARCEL_SIZE + 8,
      };
      sendHud();
    },
    say: text =>
      receive(
        { address: MOCK_OWN_ADDRESS, message: text, timestamp: Date.now() },
        true,
      ),
    setChatOpen: open => {
      chatOpen = open;
      if (open && unread > 0) {
        unread = 0;
        bus.emit(l => l.unreadChat(0));
      }
    },
  };
}
