import type { DiscoverEvent, Place } from './api';
import { remember } from './catalog';
import { parseCoordinates, normalizeKeyword } from './search';
import {
  BANNERS,
  EVENTS,
  FRIENDS,
  PLACES,
  placeById,
  SUGGESTION_WORDS,
} from './mockData';
import { matchesQuery } from './format';
import type { DiscoverApi, Page, PlaceRail, VoteState } from './types';

export type MockOptions = {
  /** Never resolves (the skeleton states): everything, or only these place rails. */
  hang?: boolean | PlaceRail[];
  /** Methods that reject. */
  fail?: (keyof DiscoverApi)[];
  /** The first card of each list gets a broken image. */
  imageFail?: boolean;
  /** More pages exist after the first. */
  hasMore?: boolean;
  noFriends?: boolean;
  favorite?: boolean;
  vote?: VoteState;
  reminder?: boolean;
  downloadMb?: number | null;
  /** Place rails that come back empty. */
  empty?: PlaceRail[];
};

const FIRST_PAGE = 6;

const breakFirst = <T extends { image: string }>(items: T[]): T[] =>
  items.map((item, i) => (i === 0 ? { ...item, image: 'mock:error' } : item));

/** Deterministic in-memory implementation for the gallery and tests. */
export function createMockApi(opts: MockOptions = {}): DiscoverApi {
  let favorite = opts.favorite ?? false;
  let vote = opts.vote ?? null;
  let reminder = opts.reminder ?? false;

  const run = <T>(method: keyof DiscoverApi, value: () => T): Promise<T> => {
    if (opts.hang === true) {
      return new Promise(() => undefined);
    }
    if (opts.fail?.includes(method)) {
      return Promise.reject(new Error(`mock ${method} failed`));
    }
    try {
      return Promise.resolve(value());
    } catch (error) {
      return Promise.reject(error);
    }
  };

  const pageOf = (list: Place[], offset: number): Page<Place> => {
    const items = list.slice(offset, offset + FIRST_PAGE);
    return {
      items: opts.imageFail && offset === 0 ? breakFirst(items) : items,
      hasMore: !!opts.hasMore || offset + FIRST_PAGE < list.length,
    };
  };

  const matching = (q: string) => {
    const coords = parseCoordinates(q);
    return coords
      ? PLACES.filter(p => p.x === coords.x && p.y === coords.y && !p.world)
      : PLACES.filter(p => matchesQuery(p.title, normalizeKeyword(q)));
  };

  const api: DiscoverApi = {
    banners: () =>
      run('banners', () => {
        const banners = opts.imageFail
          ? BANNERS.map((b, i) => (i === 0 ? { ...b, image: 'mock:error' } : b))
          : BANNERS;
        remember({ places: banners.map(b => b.place) });
        return banners;
      }),
    ftuePlaces: () => run('ftuePlaces', () => BANNERS.map(b => b.place)),
    friends: () =>
      run('friends', () => {
        const friends = opts.noFriends ? [] : FRIENDS;
        remember({ friends });
        return friends;
      }),
    events: () =>
      run('events', () => {
        const events: DiscoverEvent[] = opts.imageFail
          ? breakFirst(EVENTS)
          : EVENTS;
        remember({ events });
        return events;
      }),
    places: (rail, offset) =>
      Array.isArray(opts.hang) && opts.hang.includes(rail)
        ? new Promise(() => undefined)
        : run('places', () => {
            const lists: Record<PlaceRail, Place[]> = {
              popular: PLACES.slice(3, 13),
              new: PLACES.slice(0, 2),
              favorites: PLACES.slice(6, 8),
              mine: [placeById('mine')!],
            };
            const list = opts.empty?.includes(rail) ? [] : lists[rail];
            const page = pageOf(list, offset);
            remember({ places: page.items });
            return rail === 'popular' ? page : { ...page, hasMore: false };
          }),
    categoryPlaces: (_category, offset, search) =>
      run('categoryPlaces', () => {
        const list = [...PLACES, ...PLACES].filter(
          p => !search || matchesQuery(p.title, search),
        );
        const page = {
          items: list.slice(offset, offset + 10),
          hasMore: offset + 10 < list.length,
        };
        remember({ places: page.items });
        return page;
      }),
    search: query =>
      run('search', () => {
        const found = matching(query);
        const results = {
          featured: found.slice(0, 2),
          events: EVENTS.filter(e => matchesQuery(e.title, query)),
          mostActive: found.slice(2, 4),
          favorites: found.filter(p => ['audio', 'exodus'].includes(p.id)),
        };
        remember({ places: found, events: results.events });
        return results;
      }),
    suggest: query =>
      run('suggest', () => {
        const coords = parseCoordinates(query);
        if (coords) {
          return matching(query).map(p => ({
            kind: 'coordinates' as const,
            text: p.title,
          }));
        }
        return SUGGESTION_WORDS.filter(w =>
          w.toLowerCase().startsWith(query.trim().toLowerCase()),
        )
          .slice(0, 8)
          .map(text => ({ kind: 'keyword' as const, text }));
      }),
    placeDetail: id =>
      run('placeDetail', () => {
        const found = placeById(id);
        if (!found) {
          throw new Error(`unknown place ${id}`);
        }
        return {
          place: found,
          favorite,
          vote,
          downloadMb: opts.downloadMb === undefined ? null : opts.downloadMb,
        };
      }),
    eventDetail: id =>
      run('eventDetail', () => {
        const found = EVENTS.find(e => e.id === id);
        if (!found) {
          throw new Error(`unknown event ${id}`);
        }
        return { event: found, reminder };
      }),
    friend: address =>
      run('friend', () => FRIENDS.find(f => f.address === address) ?? null),
    setFavorite: (_id, on) =>
      run('setFavorite', () => {
        favorite = on;
      }),
    setVote: (_id, next) =>
      run('setVote', () => {
        vote = next;
      }),
    setReminder: (_id, on) =>
      run('setReminder', () => {
        reminder = on;
      }),
  };
  return api;
}

export const mockApi = createMockApi();
