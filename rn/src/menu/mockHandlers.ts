import { registerMockHandler } from '../bridge/mockHandlers';
import type { Friend as BridgeFriend } from '../bridge/methods';
import { MAIN_REALM } from '../discover/api';
import { mockNotifications, mockScene, mockSocial } from './mockData';
import type { MenuNotification, Person } from './types';

const RAW_TYPES: Record<MenuNotification['type'], string> = {
  friendship_request: 'social_service_friendship_request',
  friendship_accepted: 'social_service_friendship_accepted',
  event: 'events_started',
  reward: 'reward_assignment',
  other: 'governance_announcement',
};

const rawNotification = (n: MenuNotification, read: boolean) => ({
  id: n.id,
  type: RAW_TYPES[n.type],
  read,
  timestamp: String(n.timestamp),
  metadata: {
    title: n.subject,
    description: n.text,
    tokenName: n.subject,
    sender: { name: n.subject, profileImageUrl: n.image },
    link: n.eventId
      ? `https://decentraland.org/jump/events?id=${n.eventId}`
      : '',
  },
});

const asBridge = (p: Person): BridgeFriend => ({
  address: p.address,
  name: p.name,
  has_claimed_name: p.claimed,
  profile_picture_url: p.avatar,
});

/** Fakes scene.current, friends.* and notifications.* for the no-engine bridge (gallery, Expo Go, jest). */
export function registerMenuMockHandlers(): void {
  const requests = [...mockSocial.requests];
  const read = new Set<string>();
  const dropRequest = ({ address }: { address: string }) => {
    const at = requests.findIndex(r => r.address === address);
    if (at >= 0) {
      requests.splice(at, 1);
    }
    return {};
  };
  registerMockHandler('scene.current', () => ({
    title: mockScene.title,
    creator: mockScene.creator,
    parcel: { x: 0, y: 0 },
    realm: MAIN_REALM,
    genesis: true,
  }));
  registerMockHandler('friends.list', () => ({
    friends: mockSocial.friends.map(asBridge),
  }));
  registerMockHandler('friends.requests', () => ({
    received: requests.filter(r => r.direction === 'received').map(asBridge),
    sent: requests.filter(r => r.direction === 'sent').map(asBridge),
  }));
  registerMockHandler('friends.nearby', () => ({
    users: mockSocial.nearby.map(asBridge),
  }));
  for (const method of ['friends.accept', 'friends.reject', 'friends.cancel']) {
    registerMockHandler(method, dropRequest);
  }
  for (const method of ['friends.send', 'friends.unblock', 'friends.block']) {
    registerMockHandler(method, () => ({}));
  }
  registerMockHandler(
    'notifications.list',
    (params: { only_unread?: boolean } | undefined) => ({
      notifications: mockNotifications
        .filter(n => !(params?.only_unread && (n.read || read.has(n.id))))
        .map(n => rawNotification(n, n.read || read.has(n.id))),
    }),
  );
  registerMockHandler('notifications.read', (params: { ids: string[] }) => {
    params.ids.forEach(id => read.add(id));
    return { updated: params.ids.length };
  });
}
