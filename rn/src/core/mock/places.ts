import { Vote, type Place } from '../native';
import { createMockNotifications } from './notifications';
import type { PlacesApi } from '../api/places';

export type MockPlacesApi = Omit<
  PlacesApi,
  'notifications' | 'markNotificationsRead'
>;

const CREDITS = 120n;

const entry = (
  id: string,
  title: string,
  over: Partial<Place> = {},
): Place => ({
  id,
  title,
  creator: 'Decentraland Foundation',
  image: `mock:${id}`,
  users: 12,
  likeRate: 1,
  world: false,
  worldName: '',
  basePosition: '0,0',
  categories: [],
  ...over,
});

const PLACES: Place[] = [
  entry('genesis', 'Genesis Plaza', { users: 42, likeRate: 0.91 }),
  entry('meta', 'Meta Residence Tower - Apartments in Decentraland', {
    creator: 'Name Creator',
    basePosition: '-12,40',
    description: 'The First Skyscraper with 10 000 NFT Apartments.',
    categories: ['art', 'business', 'social'],
  }),
  entry('tower', 'Tower of madness', { creator: 'Lazaro', users: 7 }),
  entry('golf', 'GolfCraft mini golf', { basePosition: '44,-48' }),
  entry('world', 'Mmmmmmmmmmmmmmmmm', {
    world: true,
    worldName: 'mmmmmmmmmmmmmmmm.dcl.eth',
    users: 0,
  }),
  entry('bare', 'Fresh parcel', { users: 0, likeRate: undefined }),
];

/** Favorites and votes live in memory per instance, like the signed places service would keep them. */
export function createMockPlacesWithoutNotifications(): MockPlacesApi {
  let favorites: ReadonlySet<string> = new Set();
  let votes: ReadonlyMap<string, Vote> = new Map();
  let reminders: ReadonlySet<string> = new Set();

  return {
    favoritePlaces: async (offset, limit, search) => {
      const needle = search?.trim().toLowerCase();
      const rows = PLACES.filter(
        row =>
          favorites.has(row.id) &&
          (!needle || row.title.toLowerCase().includes(needle)),
      );
      return needle ? rows.slice(0, limit) : rows.slice(offset, offset + limit);
    },
    placeUserState: async placeId =>
      PLACES.some(row => row.id === placeId)
        ? {
            favorite: favorites.has(placeId),
            vote: votes.get(placeId) ?? Vote.None,
          }
        : undefined,
    setFavorite: async (placeId, value) => {
      const next = new Set(favorites);
      if (value) {
        next.add(placeId);
      } else {
        next.delete(placeId);
      }
      favorites = next;
    },
    vote: async (placeId, choice) => {
      votes = new Map(votes).set(placeId, choice);
    },
    setEventReminder: async (eventId, on) => {
      const next = new Set(reminders);
      if (on) {
        next.add(eventId);
      } else {
        next.delete(eventId);
      }
      reminders = next;
    },
    creditsBalance: async () => ({ credits: CREDITS }),
  };
}

export function createMockPlaces(): PlacesApi {
  return {
    ...createMockPlacesWithoutNotifications(),
    ...createMockNotifications(),
  };
}
