import { plural, t } from '../i18n';

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
const HOUR = 60 * MINUTE;

/** "IN 5 MINS", "IN 2 HRS", "IN 2 DAYS", or a short date a week or more out. */
export function startsIn(startsAt: number, now: number): string {
  const delta = startsAt - now;
  if (delta < MINUTE) {
    return t('EVENT_PILL_STARTING');
  }
  if (delta < HOUR) {
    const minutes = Math.round(delta / MINUTE);
    return plural('EVENT_PILL_IN_MIN', minutes, { minutes });
  }
  if (delta < 24 * HOUR) {
    const hours = Math.round(delta / HOUR);
    return plural('EVENT_PILL_IN_HR', hours, { hours });
  }
  if (delta < 7 * 24 * HOUR) {
    const days = Math.round(delta / (24 * HOUR));
    return plural('EVENT_PILL_IN_DAY', days, { days });
  }
  const date = new Date(startsAt);
  return `${MONTHS[date.getMonth()]} ${date.getDate()}`.toUpperCase();
}

export const formatRate = (rate: number | null): string =>
  rate === null ? '—' : `${Math.round(rate * 100)}%`;

export const formatUsers = (count: number): string =>
  String(count).padStart(3, '0');

export const matchesQuery = (title: string, query: string): boolean =>
  title.toLowerCase().includes(query.trim().toLowerCase());
