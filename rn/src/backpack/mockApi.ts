import { useSyncExternalStore } from 'react';
import { AvatarState } from '../events';
import type { BackpackApi, BackpackCommand, Catalog } from './api';
import { MOCK_AVATAR, MOCK_CATALOG } from './mockCatalog';

export type MockApiOptions = {
  /** `'loading'` never resolves and `'error'` rejects, for the skeleton and Connection lost states. */
  catalog?: Catalog | 'loading' | 'error';
  avatar?: Partial<AvatarState>;
};

export type MockApi = BackpackApi & {
  /** Every command the UI sent, in order. */
  calls: BackpackCommand[];
  marketplaceOpens: number;
};

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

/** Applies a command to the avatar the way the engine would; null when nothing changes. */
export function applyCommand(
  avatar: AvatarState,
  command: BackpackCommand,
  catalog: Catalog,
): AvatarState | null {
  switch (command.type) {
    case 'equip': {
      const item = catalog.wearables.find(w => same(w.id, command.urn));
      const kept = avatar.wearables.filter(
        urn =>
          !same(urn, command.urn) &&
          catalog.wearables.find(w => same(w.id, urn))?.category !==
            item?.category,
      );
      return { ...avatar, wearables: [...kept, command.urn] };
    }
    case 'unequip':
      return {
        ...avatar,
        wearables: avatar.wearables.filter(urn => !same(urn, command.urn)),
      };
    case 'set_body_shape':
      return { ...avatar, body_shape: command.urn };
    case 'set_color':
      return { ...avatar, [command.target]: command.color };
    case 'set_emote_slot':
      return {
        ...avatar,
        emotes: avatar.emotes.map((urn, i) =>
          i === command.slot ? command.urn : urn,
        ),
      };
    default:
      return null;
  }
}

/** In-memory backpack: the avatar reacts to commands, so previews and tests are interactive. */
export function createMockApi(options: MockApiOptions = {}): MockApi {
  const { catalog = MOCK_CATALOG } = options;
  let avatar: AvatarState = { ...MOCK_AVATAR, ...options.avatar };
  const listeners = new Set<() => void>();
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };
  const resolved = typeof catalog === 'object' ? catalog : MOCK_CATALOG;

  const api: MockApi = {
    calls: [],
    marketplaceOpens: 0,
    loadCatalog: () => {
      if (catalog === 'loading') {
        return new Promise<Catalog>(() => {});
      }
      return catalog === 'error'
        ? Promise.reject(new Error('mock catalog offline'))
        : Promise.resolve(catalog);
    },
    send: command => {
      api.calls.push(command);
      const next = applyCommand(avatar, command, resolved);
      if (next) {
        avatar = next;
        listeners.forEach(listener => listener());
      }
    },
    useAvatar: () =>
      useSyncExternalStore(
        subscribe,
        () => avatar,
        () => avatar,
      ),
    openMarketplace: () => {
      api.marketplaceOpens += 1;
    },
  };
  return api;
}
