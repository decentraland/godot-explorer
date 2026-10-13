import { bridgeHudApi, HudApi } from './hudApi';
import type { ChatProfile, HudBridgeEvent } from './types';

const PROGRESS_STEPS = 8;
const PROGRESS_STEP_MS = 150;
const TOTAL_RESOURCES = 584;
const CHAT_START_MS = 1600;
const CHAT_EVERY_MS = 2500;

/** Same address the mock bridge echoes sent chat from. */
export const MOCK_OWN_ADDRESS = '0xm0ck00000000000000000000000000000000000';

const profile = (
  address: string,
  name: string,
  tag: string,
  color: string,
  own = false,
): ChatProfile => ({
  address,
  name,
  tag,
  claimed: tag === '',
  color,
  own,
});

export const MOCK_PROFILES: ChatProfile[] = [
  profile(MOCK_OWN_ADDRESS, 'Mateo', '', '#FF4BED', true),
  profile(
    '0xc1a000000000000000000000000000000001f0a',
    'Clara',
    '#1f0a',
    '#FF7439',
  ),
  profile(
    '0xb0b000000000000000000000000000000002c3d2',
    'Bob',
    '#c3d2',
    '#73D3D3',
  ),
  profile('0x5a1000000000000000000000000000000003a77e', 'Sally', '', '#34CE76'),
];

const [, CLARA, BOB] = MOCK_PROFILES;

const SCRIPT: Array<{ address: string; message: string }> = [
  { address: CLARA.address, message: 'Teleported' },
  {
    address: BOB.address,
    message: 'hey! anyone going to the theatre at 12,45?',
  },
  { address: CLARA.address, message: 'check https://decentraland.org/events' },
  { address: BOB.address, message: '@Mateo welcome to Genesis Plaza' },
];

/** Replays a loading ramp and a short chat script; returns the stop function. */
export function startSimulation(
  emit: (event: HudBridgeEvent) => void,
  now = Date.now,
): () => void {
  const timers: ReturnType<typeof setTimeout>[] = [];
  const at = (ms: number, fn: () => void) => timers.push(setTimeout(fn, ms));
  for (let step = 1; step <= PROGRESS_STEPS; step += 1) {
    at(step * PROGRESS_STEP_MS, () =>
      emit({
        type: 'loading_progress',
        data: {
          loaded: Math.round((TOTAL_RESOURCES * step) / PROGRESS_STEPS),
          total: TOTAL_RESOURCES,
          mbps: 2.4,
          percent: Math.round((100 * step) / PROGRESS_STEPS),
        },
      }),
    );
  }
  SCRIPT.forEach((entry, index) =>
    at(CHAT_START_MS + index * CHAT_EVERY_MS, () =>
      emit({ type: 'chat', data: { ...entry, timestamp: now() } }),
    ),
  );
  return () => timers.forEach(clearTimeout);
}

const byAddress = (address: string) =>
  MOCK_PROFILES.find(p => p.address === address);

/** Mock engine for the HUD: the mock bridge's own events plus a loading ramp, chat script and profiles. */
export const simulatedHudApi: HudApi = {
  ...bridgeHudApi,
  subscribe(listener) {
    const stopBridge = bridgeHudApi.subscribe(listener);
    const stopSim = startSimulation(listener);
    return () => {
      stopBridge();
      stopSim();
    };
  },
  resolveProfile: async address =>
    byAddress(address) ?? (await bridgeHudApi.resolveProfile(address)),
  ownProfile: async () => MOCK_PROFILES[0],
  nearby: async () => MOCK_PROFILES.slice(1),
};
