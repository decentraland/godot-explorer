import type { ProfileApi } from '../api/profile';

/** Placeholder until this domain's mock is written: any call says so. */
export function createMockProfile(): ProfileApi {
  return new Proxy(
    {},
    {
      get: (_, name) => () => {
        throw new Error(`core mock: ${String(name)} is not mocked yet`);
      },
    },
  ) as ProfileApi;
}
