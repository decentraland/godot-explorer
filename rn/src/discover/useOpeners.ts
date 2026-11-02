import { useMemo } from 'react';
import { useNav } from '../app/NavProvider';
import { DiscoverEvent, Place } from './api';
import { remember } from './catalog';
import type { Friend } from './types';

/** Opens the place / event / friend sheets as routes; the sheet reads what was shown from the catalog. */
export function useOpeners() {
  const { push } = useNav();
  return useMemo(
    () => ({
      place: (place: Place) => {
        remember({ places: [place] });
        push({ name: 'placeDetail', params: { placeId: place.id } });
      },
      event: (event: DiscoverEvent) => {
        remember({ events: [event] });
        push({ name: 'eventDetail', params: { eventId: event.id } });
      },
      friend: (friend: Friend) => {
        remember({ friends: [friend] });
        push({ name: 'friendDetail', params: { address: friend.address } });
      },
    }),
    [push],
  );
}
