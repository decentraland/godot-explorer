import type { SceneApi } from '../api/scene';

/** Placeholder until this domain's mock is written: any call says so. */
export function createMockScene(): SceneApi {
  return new Proxy(
    {},
    {
      get: (_, name) => () => {
        throw new Error(`core mock: ${String(name)} is not mocked yet`);
      },
    },
  ) as SceneApi;
}
