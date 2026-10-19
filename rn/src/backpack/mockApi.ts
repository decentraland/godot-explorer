import { useSyncExternalStore } from 'react';
import { createMockAvatar } from '../core/mock/avatar';
import type { AvatarState, Control } from '../core/native';
import type { BackpackApi, Catalog } from './api';
import { MOCK_AVATAR, MOCK_CATALOG } from './mockCatalog';

export type MockApiOptions = {
  /** `'loading'` never resolves and `'error'` rejects, for the skeleton and Connection lost states. */
  catalog?: Catalog | 'loading' | 'error';
  avatar?: Partial<AvatarState>;
};

export type MockApi = BackpackApi & {
  /** Every command the UI sent, in order. */
  calls: Control[];
  marketplaceOpens: number;
};

/** In-memory backpack: the avatar reacts to commands, so previews and tests are interactive. */
export function createMockApi(options: MockApiOptions = {}): MockApi {
  const { catalog = MOCK_CATALOG } = options;
  const resolved = typeof catalog === 'object' ? catalog : MOCK_CATALOG;
  const avatar = createMockAvatar({
    initial: { ...MOCK_AVATAR, ...options.avatar },
    wearables: resolved.wearables,
  });
  const subscribe = (listener: () => void) => avatar.onChange(listener);
  const snapshot = () => avatar.avatarState() ?? null;

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
      avatar.applyAvatarControl(command);
    },
    useAvatar: () => useSyncExternalStore(subscribe, snapshot, snapshot),
    openMarketplace: () => {
      api.marketplaceOpens += 1;
    },
  };
  return api;
}
