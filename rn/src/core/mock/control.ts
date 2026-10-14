import type { ControlApi } from '../api/control';

/** Placeholder until this domain's mock is written: any call says so. */
export function createMockControl(): ControlApi {
  return new Proxy(
    {},
    {
      get: (_, name) => () => {
        throw new Error(`core mock: ${String(name)} is not mocked yet`);
      },
    },
  ) as ControlApi;
}
