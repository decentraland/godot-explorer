import {
  DiscoverEvent,
  eventsApi,
  featuredUrl,
  getJson,
  liveEventsUrl,
  parseEvents,
  parsePlaces,
  Place,
  placesApi,
  upcomingEventsUrl,
} from './api';
import { recallEvent, recallFriend, recallPlace, remember } from './catalog';
import { normalizeKeyword, parseCoordinates } from './search';
import type { DiscoverApi, Page, PlaceRail, VoteState } from './types';

const PAGE = 12;
const SUGGESTIONS = 8;

const pageOf = (places: Place[]): Page<Place> => ({
  items: places,
  hasMore: places.length >= PAGE,
});

const toggle = (set: Set<string>, id: string, on: boolean) => {
  if (on) {
    set.add(id);
  } else {
    set.delete(id);
  }
};

const placesUrl = (query: string) => `${placesApi()}?${query}&limit=${PAGE}`;

/**
 * Public places/events services. Friends, favorites, own places, votes and reminders need a signed
 * request, so they come back empty and apply locally; the bridge implementation wraps this with the signed ones.
 */
export function createPublicApi(
  fetchImpl: typeof fetch = (...args) => fetch(...args),
): DiscoverApi {
  const get = (url: string) => getJson(url, fetchImpl);
  const favorites = new Set<string>();
  const votes = new Map<string, VoteState>();
  const reminders = new Set<string>();

  const places = async (url: string) => {
    const found = parsePlaces(await get(url));
    remember({ places: found });
    return found;
  };
  const events = async (url: string): Promise<DiscoverEvent[]> => {
    const found = parseEvents(await get(url));
    remember({ events: found });
    return found;
  };

  const railUrls: Partial<Record<PlaceRail, string>> = {
    popular: 'order_by=most_active&order=desc',
    new: 'order_by=created_at&order=desc',
  };

  return {
    banners: async () =>
      (await places(featuredUrl())).map(place => ({
        id: place.id,
        image: place.image,
        title: place.title,
        place,
        cta: 'arrow' as const,
      })),
    friends: async () => [],
    events: async () => {
      const lists = await Promise.allSettled([
        events(liveEventsUrl()),
        events(upcomingEventsUrl()),
      ]);
      if (lists.every(list => list.status === 'rejected')) {
        throw new Error('events unavailable');
      }
      return lists.flatMap(list =>
        list.status === 'fulfilled' ? list.value : [],
      );
    },
    places: async (rail, offset) => {
      const query = railUrls[rail];
      return query
        ? pageOf(await places(placesUrl(`${query}&offset=${offset}`)))
        : { items: [], hasMore: false };
    },
    categoryPlaces: async (category, offset, search) => {
      const text = search
        ? `&search=${encodeURIComponent(normalizeKeyword(search))}`
        : '';
      return pageOf(
        await places(
          placesUrl(
            `categories=${encodeURIComponent(
              category,
            )}&offset=${offset}${text}`,
          ),
        ),
      );
    },
    search: async query => {
      const coords = parseCoordinates(query);
      const term = encodeURIComponent(normalizeKeyword(query));
      const [featured, mostActive, found] = await Promise.all([
        places(
          coords
            ? placesUrl(`positions=${coords.x},${coords.y}`)
            : placesUrl(`search=${term}&only_highlighted=true`),
        ),
        coords
          ? []
          : places(placesUrl(`search=${term}&order_by=most_active&order=desc`)),
        coords
          ? []
          : events(`${eventsApi()}?search=${term}&limit=10`).catch(() => []),
      ]);
      const seen = new Set(featured.map(p => p.id));
      return {
        featured,
        events: found,
        mostActive: mostActive.filter(p => !seen.has(p.id)),
        favorites: [],
      };
    },
    suggest: async query => {
      const coords = parseCoordinates(query);
      const url = coords
        ? placesUrl(`positions=${coords.x},${coords.y}`)
        : `${placesApi()}?search=${encodeURIComponent(
            normalizeKeyword(query),
          )}&limit=${SUGGESTIONS}`;
      const found = await places(url);
      const kind = coords ? ('coordinates' as const) : ('keyword' as const);
      return [...new Set(found.map(p => p.title))].map(text => ({
        kind,
        text,
      }));
    },
    placeDetail: async id => {
      const place =
        recallPlace(id) ??
        (await places(`${placesApi()}/${encodeURIComponent(id)}`))[0];
      if (!place) {
        throw new Error(`place ${id} not found`);
      }
      return {
        place,
        favorite: favorites.has(id),
        vote: votes.get(id) ?? null,
        downloadMb: null,
      };
    },
    eventDetail: async id => {
      const event =
        recallEvent(id) ??
        (await events(`${eventsApi()}/${encodeURIComponent(id)}`))[0];
      if (!event) {
        throw new Error(`event ${id} not found`);
      }
      return { event, reminder: reminders.has(id) };
    },
    friend: async address => recallFriend(address) ?? null,
    setFavorite: async (id, on) => {
      toggle(favorites, id, on);
    },
    setVote: async (id, vote) => {
      votes.set(id, vote);
    },
    setReminder: async (id, on) => {
      toggle(reminders, id, on);
    },
  };
}

export const publicApi = createPublicApi();
