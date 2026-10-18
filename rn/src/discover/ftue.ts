import { Image } from 'react-native';
import type { Place } from './api';
import type { DiscoverApi } from './types';

const pending = new WeakMap<DiscoverApi, Promise<Place[]>>();
const landed = new WeakMap<DiscoverApi, Place[]>();

/** The first-run list, fetched once from Avatar Create on, its card images warmed so the carousel
 * draws them at once. A failed fetch is an empty list. */
export function ftuePlaces(api: DiscoverApi): Promise<Place[]> {
  let places = pending.get(api);
  if (!places) {
    places = api
      .ftuePlaces()
      .catch((): Place[] => [])
      .then(list => {
        landed.set(api, list);
        list
          .filter(place => place.image && !place.image.startsWith('mock:'))
          .forEach(place => Image.prefetch(place.image).catch(() => false));
        return list;
      });
    pending.set(api, places);
  }
  return places;
}

/** The list once it landed, undefined while in flight: the FTUE gate never waits for it. */
export const landedFtuePlaces = (api: DiscoverApi): Place[] | undefined =>
  landed.get(api);
