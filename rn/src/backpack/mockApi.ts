import { useSyncExternalStore } from 'react';
import { createMockAvatar } from '../core/mock/avatar';
import type { AvatarState, Control } from '../core/native';
import type { BackpackApi, Catalog } from './api';
import { MOCK_AVATAR, MOCK_CATALOG } from './mockCatalog';

export type MockApiOptions = {
  /** `'loading'` never resolves and `'error'` rejects, for the skeleton and Connection lost states. */
  catalog?: Catalog | 'loading' | 'error';
  avatar?: Partial<AvatarState>;
  credits?: number;
  /** What the account owns, merged in after the catalog loads. */
  owned?: Catalog;
  /** The save on leave fails. */
  saveFails?: boolean;
};

export type MockApi = BackpackApi & {
  /** Every command the UI sent, in order. */
  calls: Control[];
  marketplaceOpens: number;
  saves: number;
};

/** In-memory backpack: the avatar reacts to commands, so previews and tests are interactive. */
export function createMockApi(options: MockApiOptions = {}): MockApi {
  const {
    catalog = MOCK_CATALOG,
    credits = 0,
    owned = { wearables: [], emotes: [] },
  } = options;
  const resolved = typeof catalog === 'object' ? catalog : MOCK_CATALOG;
  const avatar = createMockAvatar({
    initial: { ...MOCK_AVATAR, ...options.avatar },
    wearables: [...resolved.wearables, ...owned.wearables],
  });
  const subscribe = (listener: () => void) => avatar.onChange(listener);
  const snapshot = () => avatar.avatarState() ?? null;

  const api: MockApi = {
    calls: [],
    marketplaceOpens: 0,
    saves: 0,
    loadCatalog: () => {
      if (catalog === 'loading') {
        return new Promise<Catalog>(() => {});
      }
      return catalog === 'error'
        ? Promise.reject(new Error('mock catalog offline'))
        : Promise.resolve(catalog);
    },
    loadOwned: () => Promise.resolve(owned),
    saveAvatar: () => {
      api.saves += 1;
      return options.saveFails
        ? Promise.reject(new Error('mock save offline'))
        : Promise.resolve(true);
    },
    send: command => {
      api.calls.push(command);
      avatar.applyAvatarControl(command);
    },
    useAvatar: () => useSyncExternalStore(subscribe, snapshot, snapshot),
    openMarketplace: () => {
      api.marketplaceOpens += 1;
    },
    loadCredits: () => Promise.resolve(credits),
  };
  return api;
}
