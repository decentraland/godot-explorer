import { NotificationKind, NotificationSender } from '../native';
import type { Notification } from '../native';
import type { PlacesApi } from '../api/places';

const DAY = 86_400_000;

const sender = (name: string): NotificationSender => ({
  address: `0x${name.toLowerCase()}`,
  name,
  profileImageUrl: '',
  hasClaimedName: true,
});

const seed = (now: number): Notification[] => [
  {
    id: 'n1',
    timestamp: BigInt(now - 2 * DAY),
    read: false,
    kind: new NotificationKind.FriendshipAccepted({ sender: sender('Ana') }),
  },
  {
    id: 'n2',
    timestamp: BigInt(now - 2 * DAY - 1000),
    read: false,
    kind: new NotificationKind.FriendshipRequest({ sender: sender('Bo') }),
  },
  {
    id: 'n3',
    timestamp: BigInt(now - 2 * DAY - 2000),
    read: false,
    kind: new NotificationKind.Reward({
      tokenName: 'Space Jacket',
      tokenImage: '',
      title: 'New item received',
    }),
  },
  {
    id: 'n4',
    timestamp: BigInt(now - 2 * DAY - 3000),
    read: true,
    kind: new NotificationKind.EventStarted({
      title: 'Event Name',
      image: '',
      link: 'https://decentraland.org/jump/events?id=event-1',
      eventId: 'event-1',
    }),
  },
];

/** The notification half of the places slice, to be spread into the places mock. */
export function createMockNotifications(
  now: () => number = Date.now,
): Pick<PlacesApi, 'notifications' | 'markNotificationsRead'> {
  let items = seed(now());
  return {
    notifications: async (from, limit, onlyUnread) =>
      items
        .filter(n => !onlyUnread || !n.read)
        .filter(n => from === undefined || n.timestamp >= from)
        .sort((a, b) => Number(b.timestamp - a.timestamp))
        .slice(0, Math.min(limit ?? 50, 50)),
    markNotificationsRead: async ids => {
      const unread = items.filter(n => ids.includes(n.id) && !n.read);
      items = items.map(n => (ids.includes(n.id) ? { ...n, read: true } : n));
      return unread.length;
    },
  };
}
