import type { SettingsApi } from '../api/settings';

/** Placeholder until this domain's mock is written: any call says so. */
export function createMockSettings(): SettingsApi {
  return new Proxy(
    {},
    {
      get: (_, name) => () => {
        throw new Error(`core mock: ${String(name)} is not mocked yet`);
      },
    },
  ) as SettingsApi;
}
