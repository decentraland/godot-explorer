import { core } from '../core';
import { Vote, type Friend as CoreFriend } from '../core/native';
import { nameColor } from '../hud/chatModel';
import type { Place } from './api';
import { recallPlace, remember } from './catalog';
import { toPlace } from './corePlace';
import type { DiscoverApi, Friend, Page, PlaceRail, VoteState } from './types';

const PAGE = 12;
const FRIENDS = 100;

const VOTES: Record<VoteState & string, Vote> = {
  like: Vote.Up,
  dislike: Vote.Down,
};

const voteState = (vote: Vote): VoteState =>
  vote === Vote.Up ? 'like' : vote === Vote.Down ? 'dislike' : null;

const toFriend = (raw: CoreFriend): Friend => {
  const tag = raw.hasClaimedName ? undefined : `#${raw.address.slice(-4)}`;
  return {
    address: raw.address,
    name: raw.name || raw.address.slice(0, 6),
    tag,
    color: nameColor(`${raw.name}${tag ?? ''}`),
    place: null,
  };
};

const pageOf = (items: Place[]): Page<Place> => ({
  items,
  hasMore: items.length >= PAGE,
});

/** What needs the user's identity goes through the core; public listings stay with `base`. */
export function createCoreDiscoverApi(base: DiscoverApi): DiscoverApi {
  const favorites = async (offset: number, search?: string) => {
    const found = (await core().favoritePlaces(offset, PAGE, search)).map(
      toPlace,
    );
    remember({ places: found });
    return found;
  };
  const friends = async () => (await core().friends(FRIENDS, 0)).map(toFriend);
  const isWorld = (id: string) => recallPlace(id)?.world === true;

  return {
    ...base,
    friends,
    friend: async address =>
      (await base.friend(address)) ??
      (await friends()).find(f => f.address === address) ??
      null,
    places: async (rail: PlaceRail, offset) =>
      rail === 'favorites'
        ? pageOf(await favorites(offset))
        : base.places(rail, offset),
    search: async query => {
      const results = await base.search(query);
      return {
        ...results,
        favorites: await favorites(0, query.trim()).catch(() => []),
      };
    },
    placeDetail: async id => {
      const detail = await base.placeDetail(id);
      const state = await core()
        .placeUserState(id)
        .catch(() => undefined);
      return state
        ? { ...detail, favorite: state.favorite, vote: voteState(state.vote) }
        : detail;
    },
    setFavorite: (placeId, on) =>
      core().setFavorite(placeId, on, isWorld(placeId)),
    setVote: (placeId, vote) =>
      core().vote(placeId, vote ? VOTES[vote] : Vote.None, isWorld(placeId)),
    setReminder: (eventId, on) => core().setEventReminder(eventId, on),
  };
}
