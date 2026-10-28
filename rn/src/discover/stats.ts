import type { IconName } from '../ds';
import { t } from '../i18n';
import type { DiscoverEvent, Place } from './api';
import { formatDuration, recurrenceKey } from './eventTime';
import { formatRate, formatUsers } from './format';

export type StatCell = {
  kind: 'users' | 'likes' | 'location' | 'recurrence' | 'duration';
  icon: IconName;
  text: string;
};

/**
 * Cells with data only: users when someone is online, votes when rated, the location when known.
 * Three cells split the row in thirds; fewer sit at the two ends (location first when users are missing).
 */
export function placeStats(place: Place): {
  cells: StatCell[];
  layout: 'thirds' | 'ends';
} {
  const users: StatCell[] =
    place.users > 0
      ? [{ kind: 'users', icon: 'user', text: formatUsers(place.users) }]
      : [];
  const likes: StatCell[] =
    place.likeRate !== null
      ? [{ kind: 'likes', icon: 'like', text: formatRate(place.likeRate) }]
      : [];
  const location: StatCell[] = place.location
    ? [
        {
          kind: 'location',
          icon: place.world ? 'globe' : 'pin',
          text: place.location,
        },
      ]
    : [];
  const cells =
    users.length > 0
      ? [...users, ...likes, ...location]
      : [...location, ...likes];
  return { cells, layout: cells.length === 3 ? 'thirds' : 'ends' };
}

/** Recurrence, duration and (when the event has one) the location, as in the event sheet's meta row. */
export function eventMeta(event: DiscoverEvent): StatCell[] {
  const duration =
    event.endsAt !== undefined
      ? formatDuration(event.endsAt - event.startsAt)
      : null;
  const cells: StatCell[] = [
    {
      kind: 'recurrence',
      icon: 'calendar',
      text: t(recurrenceKey(event.recurrence)),
    },
  ];
  if (duration) {
    cells.push({ kind: 'duration', icon: 'clock', text: duration });
  }
  const { location, world } = event.place;
  if (location) {
    cells.push({
      kind: 'location',
      icon: world ? 'globe' : 'pin',
      text: location,
    });
  }
  return cells;
}
