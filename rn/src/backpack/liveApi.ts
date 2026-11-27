import { Linking } from 'react-native';
import type { Rarity } from '@dcl/ui-native';
import { core } from '../core';
import type { BackpackApi, Catalog } from './api';
import { loadWearables, toWearable } from './catalog';
import { DEFAULT_EMOTES, fetchEmoteThumbnails } from './emotes';
import { marketplaceUrl } from './platformPolicy';
import { useAvatar } from './useAvatar';

let cached: Promise<Catalog> | null = null;

function loadOnce(): Promise<Catalog> {
  cached ??= Promise.all([
    loadWearables(),
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
  loadOwned: async () => {
    const owned = await core().loadOwnedAvatarItems();
    return {
      wearables: owned.wearables.map(toWearable),
      emotes: owned.emotes.map(emote => ({
        id: emote.urn,
        urn: emote.urn,
        name: emote.name,
        thumbnail: emote.thumbnail,
        rarity: emote.rarity as Rarity,
        collectible: true,
      })),
    };
  },
  saveAvatar: () => core().saveAvatar(),
  send: command => core().control(command),
  useAvatar,
  openMarketplace: () => {
    Linking.openURL(marketplaceUrl()).catch(() => {});
  },
  loadCredits: () =>
    core()
      .creditsBalance()
      .then(balance => Number(balance.credits)),
};
