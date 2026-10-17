import { colors } from '../ds';
import { plural, t } from '../i18n';
import type { MenuNotification } from './types';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** "Now", "5m ago", "Yesterday", "2 days ago", "3 weeks ago"... like the legacy panel. */
export function relativeTime(timestamp: number, now: number): string {
  const delta = Math.max(0, now - timestamp);
  if (delta < MINUTE) {
    return t('NOTIFICATION_JUST_NOW');
  }
  if (delta < HOUR) {
    return t('NOTIFICATION_MINUTES_AGO', {
      minutes: Math.floor(delta / MINUTE),
    });
  }
  if (delta < DAY) {
    return t('NOTIFICATION_HOURS_AGO', { hours: Math.floor(delta / HOUR) });
  }
  const days = Math.floor(delta / DAY);
  if (days === 1) {
    return t('NOTIFICATION_YESTERDAY');
  }
  if (days < 7) {
    return plural('NOTIFICATION_DAYS_AGO', days, { days });
  }
  if (days < 30) {
    const weeks = Math.floor(days / 7);
    return plural('NOTIFICATION_WEEKS_AGO', weeks, { weeks });
  }
  if (days < 365) {
    const months = Math.floor(days / 30);
    return plural('NOTIFICATION_MONTHS_AGO', months, { months });
  }
  const years = Math.floor(days / 365);
  return plural('NOTIFICATION_YEARS_AGO', years, { years });
}

export type NotificationView = {
  title: string;
  body: string;
  titleColor: string;
  /** Person avatars are circles, event and item images are rounded squares. */
  round: boolean;
};

const SOCIAL = '#A0ABFF';
const EVENT = '#73FFAF';
const REWARD = '#DF9CFF';

export function notificationView(n: MenuNotification): NotificationView {
  switch (n.type) {
    case 'friendship_accepted':
      return {
        title: n.subject,
        body: t('NOTIF_BODY_ACCEPT_YOUR_FRIEND_REQUEST'),
        titleColor: SOCIAL,
        round: true,
      };
    case 'friendship_request':
      return {
        title: n.subject,
        body: t('NOTIF_BODY_WANT_TO_BE_YOUR_FRIEND'),
        titleColor: SOCIAL,
        round: true,
      };
    case 'event':
      return {
        title: n.subject,
        body: t('NOTIF_BODY_EVENT_STARTED'),
        titleColor: EVENT,
        round: false,
      };
    case 'reward':
      return {
        title: t('NOTIF_HEADER_NEW_ITEM_RECEIVED'),
        body: n.subject,
        titleColor: REWARD,
        round: false,
      };
    default:
      return {
        title: n.subject || t('NOTIF_HEADER_NOTIFICATION'),
        body: n.text ?? '',
        titleColor: colors.snow,
        round: false,
      };
  }
}

export const unreadCount = (items: MenuNotification[]): number =>
  items.filter(n => !n.read).length;

export function markAllRead(items: MenuNotification[]): MenuNotification[] {
  return items.map(n => (n.read ? n : { ...n, read: true }));
}

export function markRead(
  items: MenuNotification[],
  id: string,
): MenuNotification[] {
  return items.map(n => (n.id === id && !n.read ? { ...n, read: true } : n));
}

export type NotificationTarget =
  | { section: 'social'; tab: 'friends' | 'requests' }
  | { section: 'backpack' }
  | { eventId: string }
  | null;

/** Where tapping a card leads: the social tab, the backpack, or the event modal. */
export function notificationTarget(n: MenuNotification): NotificationTarget {
  switch (n.type) {
    case 'friendship_request':
      return { section: 'social', tab: 'requests' };
    case 'friendship_accepted':
      return { section: 'social', tab: 'friends' };
    case 'reward':
      return { section: 'backpack' };
    case 'event':
      return n.eventId ? { eventId: n.eventId } : null;
    default:
      return null;
  }
}
