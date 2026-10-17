import { Linking } from 'react-native';
import type { GodotCommand } from '../GodotBridge';
import { ensureEngine, send, useEngine } from '../engine';
import type { BackpackApi, Catalog } from './api';
import { fetchWearables } from './catalog';
import { DEFAULT_EMOTES, fetchEmoteThumbnails } from './emotes';
import { MARKETPLACE_URL } from './platformPolicy';

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

/** Talks to the catalyst and, through the bridge, to Godot. */
export const liveApi: BackpackApi = {
  loadCatalog: () => {
    ensureEngine();
    return loadOnce();
  },
  send: command => send(command as GodotCommand),
  useAvatar: () => useEngine().avatar,
  openMarketplace: () => {
    Linking.openURL(MARKETPLACE_URL).catch(() => {});
  },
};
