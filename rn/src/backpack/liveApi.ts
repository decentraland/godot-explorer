import { Linking } from 'react-native';
import { core } from '../core';
import type { BackpackApi, Catalog } from './api';
import { fetchWearables } from './catalog';
import { DEFAULT_EMOTES, fetchEmoteThumbnails } from './emotes';
import { marketplaceUrl } from './platformPolicy';
import { useAvatar } from './useAvatar';

let cached: Promise<Catalog> | null = null;

function loadOnce(): Promise<Catalog> {
  cached ??= Promise.all([
    fetchWearables(),
    fetchEmoteThumbnails().catch(() => DEFAULT_EMOTES),
  ])
    .then(([wearables, emotes]) => ({ wearables, emotes }))
    .catch(error => {
      cached = null;
      throw error;
    });
  return cached;
}

/** Reads the catalyst and drives the avatar through the core. */
export const liveApi: BackpackApi = {
  loadCatalog: loadOnce,
  send: command => core().control(command),
  useAvatar,
  openMarketplace: () => {
    Linking.openURL(marketplaceUrl()).catch(() => {});
  },
};
