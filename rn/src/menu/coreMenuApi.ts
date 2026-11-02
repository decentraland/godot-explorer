import { core } from '../core';
import { fetchDiscover, mainRealm } from '../discover/api';
import { deviceDetails } from '../platform/deviceFacts';
import { createCoreNotificationsApi } from './coreNotificationsApi';
import { linkQueryFor } from './api';
import type { DiscoverFeed, MenuApi, SceneInfo } from './types';

const safeVersion = (): string => {
  try {
    return core().version();
  } catch {
    return '';
  }
};

const FALLBACK_SCENE: SceneInfo = {
  title: 'Decentraland',
  creator: '',
  linkQuery: 'position=0,0',
};

/**
 * Scene, screenshots and notifications come from the core, the discover feed from the public
 * places/events APIs; friends stay with `base` (mock data until friends are
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
    submitBugReport: async report => {
      const device = await deviceDetails();
      await core().submitBugReport({
        category: report.category,
        description: report.description,
        screenshots: report.screenshots,
        operatingSystem: device.osVersion,
        device: `${device.brand} ${device.model}`.trim(),
        graphicsCard: device.socModel || device.hardware,
        ramMb: BigInt(Math.round(device.totalRamMb)),
        clientVersion: safeVersion(),
        coordinates: '',
      });
    },
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
