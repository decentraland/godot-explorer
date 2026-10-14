import type { SocialApi } from '../api/social';

/** Placeholder until this domain's mock is written: any call says so. */
export function createMockSocial(): SocialApi {
  return new Proxy(
    {},
    {
      get: (_, name) => () => {
        throw new Error(`core mock: ${String(name)} is not mocked yet`);
      },
    },
  ) as SocialApi;
}
