import type { DiscoverEvent, Place } from '../discover/api';
import { mockUrls } from '../core/mock/gates';
import type {
  Friend,
  FriendRequest,
  MenuNotification,
  Person,
  SceneInfo,
  SocialData,
} from './types';

const person = (n: number, over: Partial<Person> = {}): Person => ({
  address: `0x${String(n).padStart(40, '0')}`,
  name: 'PlayerName',
  claimed: true,
  avatar: '',
  color: '#73D3D3',
  place: 'Place Name',
  ...over,
});

export const mockFriends: Friend[] = [
  { ...person(1), status: 'online' },
  { ...person(2), status: 'online', samePlace: true, place: undefined },
  { ...person(3), status: 'offline' },
  { ...person(4), status: 'offline', place: undefined },
  { ...person(5, { claimed: false }), status: 'blocked', place: undefined },
  { ...person(6), status: 'blocked', place: undefined },
];

export const mockRequests: FriendRequest[] = [
  { ...person(7), direction: 'received' },
  { ...person(8), direction: 'received' },
  { ...person(9), direction: 'sent' },
];

export const mockNearby: Person[] = Array.from({ length: 10 }, (_, i) =>
  person(10 + i),
);

export const mockSocial: SocialData = {
  friends: mockFriends,
  requests: mockRequests,
  nearby: mockNearby,
};

export const mockScene: SceneInfo = {
  title: 'Genesis Plaza PlazaPlaza',
  creator: 'Lazaro',
  linkQuery: 'position=0,0',
};

const place = (n: number, title: string): Place => ({
  id: `place-${n}`,
  title,
  creator: 'Lazaro',
  image: '',
  users: 0,
  likeRate: 1,
  location: '0,0',
  realm: mockUrls().mainRealm,
  x: 0,
  y: 0,
});

export const mockFeatured: Place[] = [
  place(1, 'Genesis Plaza'),
  place(2, 'Meta Residence Tower'),
  place(3, 'Space Runner'),
];

const NOW = Date.UTC(2026, 9, 12, 12, 0, 0);
const event = (n: number, title: string, live: boolean, mins: number) => ({
  id: `event-${n}`,
  title,
  creator: n === 1 ? 'Lazaro' : 'Crypt0M1',
  image: '',
  live,
  trending: live,
  attendees: live ? 12 : 0,
  startsAt: NOW + mins * 60_000,
  place: { ...place(10 + n, title), title },
});

export const mockEvents: DiscoverEvent[] = [
  event(1, 'Experience Name two lines example', true, -30),
  event(2, 'CRYPTOM1NOTAUR - THURSDAY', false, 5),
  event(3, 'DCL Japan Meetup', false, 30),
];

export const MOCK_NOW = NOW;

const note = (
  id: string,
  type: MenuNotification['type'],
  subject: string,
  ageMs: number,
  read = false,
  extra: Partial<MenuNotification> = {},
): MenuNotification => ({
  id,
  type,
  read,
  subject,
  timestamp: NOW - ageMs,
  image: '',
  ...extra,
});

const DAY = 86_400_000;
export const mockNotifications: MenuNotification[] = [
  note('n1', 'friendship_accepted', 'PlayerName', 2 * DAY, false, {
    avatarColor: '#9D8CFF',
  }),
  note('n2', 'friendship_request', 'PlayerName', 2 * DAY, false, {
    avatarColor: '#9D8CFF',
  }),
  note('n3', 'reward', 'Space Jacket', 2 * DAY),
  note('n4', 'event', 'Event Name', 2 * DAY, true),
  note('n5', 'other', '', 3 * DAY, true, {
    text: 'Voting has ended',
  }),
];
