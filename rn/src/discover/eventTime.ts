import { core } from '../core';
import { I18nKey, t } from '../i18n';
import type { DiscoverEvent, Place } from './api';

const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
];
const MINUTE = 60_000;
const pad = (n: number) => String(n).padStart(2, '0');

export function recurrenceKey(r: DiscoverEvent['recurrence']): I18nKey {
  switch (r) {
    case 'daily':
      return 'EVENT_FREQUENCY_DAILY';
    case 'weekly':
      return 'EVENT_FREQUENCY_WEEKLY';
    case 'monthly':
      return 'EVENT_FREQUENCY_MONTHLY';
    default:
      return 'EVENT_FREQUENCY_ONCE';
  }
}

/** "1 Hr", "30 Min", "1 Hr 30 Min"; null for a missing or empty span. */
export function formatDuration(ms: number): string | null {
  const minutes = Math.round(ms / MINUTE);
  if (!(minutes > 0)) {
    return null;
  }
  const parts = [];
  if (minutes >= 60) {
    parts.push(t('EVENT_DURATION_HR', { n: Math.floor(minutes / 60) }));
  }
  if (minutes % 60 > 0) {
    parts.push(t('EVENT_DURATION_MIN', { n: minutes % 60 }));
  }
  return parts.join(' ');
}

const clock = (d: Date) => {
  const h = d.getUTCHours();
  return `${pad(h % 12 || 12)}:${pad(d.getUTCMinutes())}${
    h < 12 ? 'am' : 'pm'
  }`;
};

/** `offsetMin` is Date#getTimezoneOffset (minutes west of UTC), so UTC-3 is 180. */
export function formatSchedule(start: number, end: number, offsetMin: number) {
  const from = new Date(start - offsetMin * MINUTE);
  const to = new Date(end - offsetMin * MINUTE);
  const east = -offsetMin;
  const sign = east < 0 ? '-' : '+';
  const abs = Math.abs(east);
  const zone = `(UTC${sign}${Math.floor(abs / 60)}${
    abs % 60 ? `:${pad(abs % 60)}` : ''
  })`;
  return t('CALENDAR_TIME_RANGE', {
    start: `${DAYS[from.getUTCDay()]}, ${
      MONTHS[from.getUTCMonth()]
    } ${from.getUTCDate()}. ${clock(from)}`,
    end: clock(to),
    tz: zone,
  });
}

const stamp = (ms: number) =>
  new Date(ms).toISOString().replace(/[-:]|\.\d{3}/g, '');

/** Add-to-calendar link; the OS opens it in the calendar app or the browser. */
export function calendarUrl(e: {
  title: string;
  description: string;
  location: string;
  start: number;
  end: number;
}): string {
  const q = (k: string, v: string) => `${k}=${encodeURIComponent(v)}`;
  return `https://calendar.google.com/calendar/render?${[
    q('action', 'TEMPLATE'),
    q('text', e.title),
    q('dates', `${stamp(e.start)}/${stamp(e.end)}`),
    q('details', e.description),
    q('location', e.location),
  ].join('&')}`;
}

export const placeJumpLink = (place: Place): string =>
  place.world
    ? core().jumpRealmUrl(place.location)
    : core().jumpPositionUrl(place.location);

export const eventJumpLink = (id: string): string => core().eventUrl(id);
