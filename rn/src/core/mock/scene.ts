import type { CurrentScene } from '../native';
import type { SceneApi } from '../api/scene';
import { mockBus } from './emitter';
import type { MockBus } from './emitter';

const SCREENSHOT_PATH = '/tmp/mock-screenshot.png';

/** The scene the player stands in, a screenshot path and analytics that go nowhere. */
export function createMockScene(bus: MockBus = mockBus): SceneApi {
  return {
    currentScene: async (): Promise<CurrentScene> => {
      const state = bus.state();
      const inWorld = state.tag === 'InWorld';
      return {
        title: 'Genesis Plaza',
        creator: 'Decentraland',
        parcel: inWorld ? state.inner.parcel : { x: 0, y: 0 },
        realm: inWorld ? state.inner.realm : 'main',
        genesis: !inWorld || state.inner.realm === 'main',
      };
    },
    captureScreenshot: async () => SCREENSHOT_PATH,
    track: () => {},
    trackScreen: () => {},
    setAnalyticsConsent: async () => {},
    flushAnalytics: async () => {},
    setAnalyticsFlushInterval: () => {},
  };
}
