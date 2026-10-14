import type { GatesApi } from '../api/gates';

/** Placeholder until this domain's mock is written: any call says so. */
export function createMockGates(): GatesApi {
  return new Proxy(
    {},
    {
      get: (_, name) => () => {
        throw new Error(`core mock: ${String(name)} is not mocked yet`);
      },
    },
  ) as GatesApi;
}
