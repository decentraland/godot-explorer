import { ColorTarget, Control_Tags } from '../native';
import type { AvatarState, Control } from '../native';
import type { ControlApi } from '../api/control';
import type { Wearable } from '../../backpack/catalog';

const BASE = 'urn:decentraland:off-chain:base-avatars:';
const MALE = `${BASE}BaseMale`;
const FEMALE = `${BASE}BaseFemale`;
const EMOTE_SLOTS = 10;

const DEFAULT_AVATAR: AvatarState = {
  bodyShape: FEMALE,
  wearables: [],
  emotes: Array<string>(EMOTE_SLOTS).fill(''),
  skin: '#cc9b78',
  hair: '#3c2415',
  eyes: '#38c4d4',
};

const COLORS = {
  skin: ['#ffe4c6', '#d9a982', '#a56f4a', '#6b3f28'],
  hair: ['#1c1c1c', '#5a3410', '#8a2515', '#e0952c'],
  eyes: ['#2b2b2b', '#6b4a2e', '#2f6fb0', '#2f6f4a'],
};
const COLOR_KEYS = {
  [ColorTarget.Skin]: 'skin',
  [ColorTarget.Hair]: 'hair',
  [ColorTarget.Eyes]: 'eyes',
} as const;
const OPTIONAL_CHANCE = 0.6;
const REQUIRED = ['body_shape', 'eyes', 'eyebrows', 'mouth'];
export const isRemovable = (category: string) => !REQUIRED.includes(category);

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

export type MockAvatarOptions = {
  initial?: AvatarState;
  /** What the engine resolves urns against: equip replaces the same category, randomize draws from it. */
  wearables?: Wearable[];
  random?: () => number;
};

export type MockAvatar = Pick<ControlApi, 'avatarState'> & {
  /** Applies an avatar command the way the engine would; other commands change nothing. */
  applyAvatarControl(control: Control): void;
  /** Called after every change, for the mock core to emit `avatarChanged`. */
  onChange(listener: (avatar: AvatarState) => void): () => void;
  /** What equip resolves categories against; the mock core sets it when the catalog loads. */
  setWearables(wearables: Wearable[]): void;
};

const pick = <T>(items: T[], random: () => number): T =>
  items[Math.floor(random() * items.length)];

function randomized(
  avatar: AvatarState,
  wearables: Wearable[],
  random: () => number,
): AvatarState {
  const bodyShape = random() < 0.5 ? MALE : FEMALE;
  const fitting = wearables.filter(
    w =>
      w.category !== 'body_shape' &&
      w.rarity === 'base' &&
      (w.bodyShapes.length === 0 ||
        w.bodyShapes.some(shape => same(shape, bodyShape))),
  );
  const categories = Array.from(new Set(fitting.map(w => w.category)));
  const worn = categories.flatMap(category => {
    const choices = fitting.filter(w => w.category === category);
    const wanted = !isRemovable(category) || random() < OPTIONAL_CHANCE;
    return wanted ? [pick(choices, random).id] : [];
  });
  return {
    bodyShape,
    wearables: worn,
    emotes: avatar.emotes,
    skin: pick(COLORS.skin, random),
    hair: pick(COLORS.hair, random),
    eyes: pick(COLORS.eyes, random),
  };
}

/** The next avatar after a command, or the same object when nothing changes. */
export function reduceAvatar(
  avatar: AvatarState,
  control: Control,
  wearables: Wearable[],
  random: () => number,
): AvatarState {
  switch (control.tag) {
    case Control_Tags.Equip: {
      const { urn } = control.inner;
      const category = wearables.find(w => same(w.id, urn))?.category;
      const kept = avatar.wearables.filter(
        worn =>
          !same(worn, urn) &&
          (category === undefined ||
            wearables.find(w => same(w.id, worn))?.category !== category),
      );
      return { ...avatar, wearables: [...kept, urn] };
    }
    case Control_Tags.Unequip:
      return {
        ...avatar,
        wearables: avatar.wearables.filter(
          worn => !same(worn, control.inner.urn),
        ),
      };
    case Control_Tags.SetBodyShape:
      return { ...avatar, bodyShape: control.inner.urn };
    case Control_Tags.SetColor: {
      const { target, color } = control.inner;
      return { ...avatar, [COLOR_KEYS[target]]: color };
    }
    case Control_Tags.SetEmoteSlot: {
      const { slot, urn } = control.inner;
      return {
        ...avatar,
        emotes: avatar.emotes.map((current, i) => (i === slot ? urn : current)),
      };
    }
    case Control_Tags.RandomizeAvatar:
      return randomized(avatar, wearables, random);
    default:
      return avatar;
  }
}

/** An in-memory equipped avatar for builds without the engine. */
export function createMockAvatar(options: MockAvatarOptions = {}): MockAvatar {
  const { initial = DEFAULT_AVATAR, random = Math.random } = options;
  let wearables = options.wearables ?? [];
  let avatar = initial;
  const listeners = new Set<(avatar: AvatarState) => void>();
  return {
    avatarState: () => avatar,
    applyAvatarControl: control => {
      const next = reduceAvatar(avatar, control, wearables, random);
      if (next !== avatar) {
        avatar = next;
        listeners.forEach(listener => listener(next));
      }
    },
    setWearables: next => {
      wearables = next;
    },
    onChange: listener => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
