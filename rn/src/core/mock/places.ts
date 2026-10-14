import type { PlacesApi } from '../api/places';

/** Placeholder until this domain's mock is written: any call says so. */
export function createMockPlaces(): PlacesApi {
  return new Proxy(
    {},
    {
      get: (_, name) => () => {
        throw new Error(`core mock: ${String(name)} is not mocked yet`);
      },
    },
  ) as PlacesApi;
}
