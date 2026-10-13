import type { DiscoverEvent, Place } from './api';
import { MAIN_REALM, worldRealm } from './api';
import type { Banner, Friend } from './types';

/** `mock:<n>` images draw a deterministic gradient so previews need no network. */
export const MOCK_DESCRIPTION =
  'Meta Residence Tower - The First Skyscraper with 10 000 NFT Apartments in Decentraland. Own your home in Metaverse. No-code solution, Powerful editor, more than 700+ units of furniture, floors, wallpapers, ceilings. Advanced Access to Apartments. And much more. Join our\ndiscord: https://discordzgg/fgwHmvpDwf\nTwitter: https://twitter.com/residence_meta';

const place = (
  id: string,
  title: string,
  image: number | 'error',
  over: Partial<Place> = {},
): Place => ({
  id,
  title,
  creator: 'Decentraland Foundation',
  image: `mock:${image}`,
  users: 12,
  likeRate: 1,
  location: '0,0',
  realm: MAIN_REALM,
  x: 0,
  y: 0,
  world: false,
  ...over,
});

export const PLACES: Place[] = [
  place('genesis', 'Genesis Plaza', 0, { users: 42, likeRate: 0.91 }),
  place('meta', 'Meta Residence Tower - Apartments in Decentraland', 1, {
    creator: 'Name Creator',
    location: '-12,40',
    x: -12,
    y: 40,
    description: MOCK_DESCRIPTION,
    categories: ['art', 'business', 'social'],
  }),
  place('tower', 'Tower of madness', 2, { creator: 'Lazaro', users: 7 }),
  place('wonder', 'WonderMine Crafting Game DCL', 3, {
    users: 5,
    likeRate: 0.8,
  }),
  place('dejay', '. dejay Bar & Restaurant', 4, { users: 9 }),
  place('butterfly', 'Butterfly Prawn Farming', 5, { users: 3 }),
  place('audio', 'Audio Hot Lab', 6, { users: 2, likeRate: 0.95 }),
  place('exodus', 'Exodus (casa)', 7, { users: 1 }),
  place('casai', 'Casai Knights of Antrom', 8, { users: 4 }),
  place('rousto', 'Casa Rousto', 9, { users: 6 }),
  place('crystal', 'Casa Crystal City', 10, { users: 2 }),
  place('golf', 'GolfCraft mini golf', 11, {
    location: '44,-48',
    x: 44,
    y: -48,
  }),
  place('world', 'Mmmmmmmmmmmmmmmmm', 12, {
    world: true,
    location: 'mmmmmmmmmmmmmmmm.dcl.eth',
    realm: worldRealm('mmmmmmmmmmmmmmmm.dcl.eth'),
    users: 0,
    likeRate: 1,
  }),
  place('genesis-lite', 'Genesis Plaza', 0, { users: 0, likeRate: 1 }),
  place('bare', 'Fresh parcel', 13, { users: 0, likeRate: null }),
  place('mine', 'My place name', 14, {
    creator: 'You',
    users: 0,
    likeRate: null,
  }),
];

export const placeById = (id: string): Place | undefined =>
  PLACES.find(p => p.id === id);

const HOUR = 3_600_000;
export const MOCK_NOW = Date.parse('2026-10-12T15:00:00.000Z');

const event = (
  id: string,
  title: string,
  image: number,
  over: Partial<DiscoverEvent> = {},
): DiscoverEvent => ({
  id,
  title,
  creator: 'Lazaro',
  image: `mock:${image}`,
  live: false,
  trending: false,
  attendees: 14,
  startsAt: MOCK_NOW + 5 * 60_000,
  endsAt: MOCK_NOW + 5 * 60_000 + HOUR,
  recurrence: 'weekly',
  description:
    '🧋 Chiri Storytelling Community\n🛠 Chiri Builds\nStreams to learn how to create 3D Low Poly assets ready for Decentraland. We will cover the full pipeline: Idea, low poly modeling, texturing, colliders, GLB export, and importing into the Creator Hub 💻\nEach event will share a different process\n✨ Asset planning & references\n✨ Low poly modeling of an Asset\n✨ Export GLB + Creator Hub import\nOur community is inclusive, kind, and playful 🌈',
  schedule: [0, 7, 14].map(d => ({
    start: MOCK_NOW + d * 24 * HOUR,
    end: MOCK_NOW + d * 24 * HOUR + 2 * HOUR,
  })),
  place: place(`event-${id}`, title, image, {
    location: '-35,-31',
    x: -35,
    y: -31,
    users: 0,
    likeRate: null,
  }),
  ...over,
});

export const EVENTS: DiscoverEvent[] = [
  event('chiri', '🛠 Chiri Builds', 15, {
    live: true,
    trending: true,
    startsAt: MOCK_NOW - HOUR,
    endsAt: MOCK_NOW + HOUR,
  }),
  event('crypto', 'CRYPTOMINOTAURS Party', 16, {
    creator: 'Crypt0M1',
    featured: true,
  }),
  event('fashion', 'Casa Fashion', 17, {
    creator: 'Hancos',
    startsAt: MOCK_NOW + 30 * 60_000,
    endsAt: MOCK_NOW + 90 * 60_000,
  }),
  event('she', 'Casa SheFi Salon', 18, {
    creator: 'Msjoedor',
    recurrence: 'once',
    startsAt: MOCK_NOW + 2 * 24 * HOUR,
    endsAt: MOCK_NOW + 2 * 24 * HOUR + HOUR,
  }),
  event('world-event', 'Experience in a world', 19, {
    live: true,
    place: place('event-world', 'Experience in a world', 19, {
      world: true,
      location: 'aaaaaaaaaaaaaaaa.dcl.eth',
      realm: worldRealm('aaaaaaaaaaaaaaaa.dcl.eth'),
      users: 0,
      likeRate: null,
    }),
  }),
  event('nowhere', 'Weekly meetup', 20, {
    place: place('event-nowhere', 'Weekly meetup', 20, {
      location: '',
      users: 0,
      likeRate: null,
    }),
  }),
];

export const eventById = (id: string) => EVENTS.find(e => e.id === id);

const FRIEND_COLORS = ['#F1C7F6', '#FFF18A', '#C8F95B', '#9BE3FF', '#FFC48A'];
export const FRIENDS: Friend[] = [
  'User p name',
  'Username',
  'Username',
  'Ana',
  'Beto',
].map((name, i) => ({
  address: `0x${i}`,
  name,
  tag: i === 0 ? '#1234' : undefined,
  color: FRIEND_COLORS[i],
  place: PLACES[i % 3],
}));

export const BANNERS: Banner[] = [
  {
    id: 'b1',
    image: 'mock:0',
    title: 'Experience Name 2 lines example elipsis name',
    place: PLACES[0],
    cta: 'arrow',
  },
  { id: 'b2', image: 'mock:2', title: '', place: PLACES[2], cta: 'button' },
  {
    id: 'b3',
    image: 'mock:1',
    title: 'Meta Residence Tower',
    place: PLACES[1],
    cta: 'arrow',
  },
];

export const SUGGESTION_WORDS = [
  'Caos',
  'Casa',
  'Casas',
  'Casa Azul',
  'Casa Py',
  'Casa Rousto',
  'Casa Sur',
  'Casa True',
  'Casa Z',
  'Casa Zombie',
];
