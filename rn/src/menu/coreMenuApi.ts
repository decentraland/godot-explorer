import { core } from '../core';
import { fetchDiscover, mainRealm } from '../discover/api';
import { createCoreNotificationsApi } from './coreNotificationsApi';
import { linkQueryFor } from './api';
import type { DiscoverFeed, MenuApi, SceneInfo } from './types';

const FALLBACK_SCENE: SceneInfo = {
  title: 'Decentraland',
  creator: '',
  linkQuery: 'position=0,0',
};

/**
 * Scene, screenshots and notifications come from the core, the discover feed from the public
 * places/events APIs; friends and bug reports stay with `base` (mock data until friends are
 * reimplemented in TypeScript, 6.9).
 */
export function createCoreMenuApi(
  base: MenuApi,
  fetchImpl: typeof fetch = fetch,
): MenuApi {
  return {
    ...base,
    scene: async (): Promise<SceneInfo> => {
      try {
        const { title, creator, parcel, realm, genesis } =
          await core().currentScene();
        return {
          title: title || FALLBACK_SCENE.title,
          creator,
          linkQuery: linkQueryFor(
            genesis ? mainRealm() : realm,
            parcel.x,
            parcel.y,
          ),
        };
      } catch {
        return FALLBACK_SCENE;
      }
    },
    captureScreenshot: async () => `file://${await core().captureScreenshot()}`,
    discover: async (): Promise<DiscoverFeed> => {
      const { featured, events } = await fetchDiscover(fetchImpl);
      return {
        featured,
        events,
        featuredEvents: events.filter(e => e.featured).map(e => e.id),
      };
    },
    ...createCoreNotificationsApi(),
  };
}
