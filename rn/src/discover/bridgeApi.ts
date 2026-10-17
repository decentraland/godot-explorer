import { request } from '../bridge/request';
import type { Friend as BridgeFriend } from '../bridge/methods';
import { nameColor } from '../hud/chatModel';
import { EVENTS_API, parsePlaces, Place, PLACES_API } from './api';
import { recallPlace, remember } from './catalog';
import type {
  DiscoverApi,
  Friend,
  Page,
  PlaceDetail,
  PlaceRail,
  VoteState,
} from './types';

const PAGE = 12;

const VOTES = { like: 'up', dislike: 'down' } as const;

type Raw = Record<string, unknown>;

/** The places service answers `{ok, data}` with an array for lists and an object for one place. */
const asList = (body: unknown): unknown => {
  const data = (body as { data?: unknown } | null)?.data;
  return { data: Array.isArray(data) ? data : data ? [data] : [] };
};

const toFriend = (raw: BridgeFriend): Friend => {
  const tag = raw.has_claimed_name ? undefined : `#${raw.address.slice(-4)}`;
  return {
    address: raw.address,
    name: raw.name || raw.address.slice(0, 6),
    tag,
    color: nameColor(`${raw.name}${tag ?? ''}`),
    place: null,
  };
};

/**
 * What needs the user's identity goes through the Godot bridge (`fetch.signed`, `places.*`);
 * public listings stay with `base`.
 */
export function createBridgeDiscoverApi(base: DiscoverApi): DiscoverApi {
  const signedPlaces = async (query: string): Promise<Place[]> => {
    const { body } = await request('fetch.signed', {
      url: `${PLACES_API}?${query}`,
    });
    const found = parsePlaces(asList(body));
    remember({ places: found });
    return found;
  };
  const pageOf = (items: Place[]): Page<Place> => ({
    items,
    hasMore: items.length >= PAGE,
  });
  const isWorld = (id: string) => recallPlace(id)?.world === true;

  return {
    ...base,
    friends: async () => {
      const { friends } = await request('friends.list');
      return friends.map(toFriend);
    },
    friend: async address => {
      const known = await base.friend(address);
      if (known) {
        return known;
      }
      const { friends } = await request('friends.list');
      const found = friends.find(f => f.address === address);
      return found ? toFriend(found) : null;
    },
    places: async (rail: PlaceRail, offset) =>
      rail === 'favorites'
        ? pageOf(
            await signedPlaces(
              `only_favorites=true&limit=${PAGE}&offset=${offset}`,
            ),
          )
        : base.places(rail, offset),
    search: async query => {
      const results = await base.search(query);
      const favorites = await signedPlaces(
        `only_favorites=true&limit=${PAGE}&search=${encodeURIComponent(
          query.trim(),
        )}`,
      ).catch(() => []);
      return { ...results, favorites };
    },
    placeDetail: async id => {
      const detail = await base.placeDetail(id);
      // Favorite and vote are per user, so they come from a signed read of the place.
      const { body } = await request('fetch.signed', {
        url: `${PLACES_API}/${encodeURIComponent(id)}`,
      }).catch(() => ({ body: null }));
      const raw = (asList(body) as { data: Raw[] }).data[0];
      if (!raw) {
        return detail;
      }
      const vote: VoteState = raw.user_like
        ? 'like'
        : raw.user_dislike
        ? 'dislike'
        : null;
      return {
        ...detail,
        favorite: raw.user_favorite === true,
        vote,
      } satisfies PlaceDetail;
    },
    setFavorite: async (placeId, on) => {
      await request('places.favorite', {
        place_id: placeId,
        value: on,
        is_world: isWorld(placeId),
      });
    },
    setVote: async (placeId, vote) => {
      await request('places.vote', {
        place_id: placeId,
        vote: vote ? VOTES[vote] : 'none',
        is_world: isWorld(placeId),
      });
    },
    setReminder: async (eventId, on) => {
      await request('fetch.signed', {
        url: `${EVENTS_API}/${encodeURIComponent(eventId)}/attendees`,
        method: on ? 'POST' : 'DELETE',
        body: '{}',
      });
    },
  };
}
