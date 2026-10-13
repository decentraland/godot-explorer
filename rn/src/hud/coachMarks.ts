/** Minimal async key-value store; wave 3 backs it with the app's persistent storage. */
export type HudStorage = {
  getItem(key: string): Promise<string | null>;
  setItem(key: string, value: string): Promise<void>;
};

export const COACH_STEPS = ['controls', 'camera'] as const;
export type CoachStep = (typeof COACH_STEPS)[number];

const SEEN_KEY = 'hud.coachMarksSeen';

export function createMemoryStorage(
  initial: Record<string, string> = {},
): HudStorage {
  const data = new Map(Object.entries(initial));
  return {
    getItem: async key => data.get(key) ?? null,
    setItem: async (key, value) => {
      data.set(key, value);
    },
  };
}

export async function loadCoachSeen(storage: HudStorage): Promise<boolean> {
  try {
    return (await storage.getItem(SEEN_KEY)) === '1';
  } catch {
    return false;
  }
}

export async function saveCoachSeen(storage: HudStorage): Promise<void> {
  try {
    await storage.setItem(SEEN_KEY, '1');
  } catch {
    // Not persisting only means the tour shows again next session.
  }
}

let defaultStorage: HudStorage = createMemoryStorage();

export const getHudStorage = (): HudStorage => defaultStorage;

export function setHudStorage(storage: HudStorage): void {
  defaultStorage = storage;
}
