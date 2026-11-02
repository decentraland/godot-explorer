import { core } from '../core';
import { Vote } from '../core/native';
import type { Place } from './api';
import { recallPlace, remember } from './catalog';
import { FRIENDS } from './mockData';
import { toPlace } from './corePlace';
import type { DiscoverApi, Page, PlaceRail, VoteState } from './types';

const PAGE = 12;

const VOTES: Record<VoteState & string, Vote> = {
  like: Vote.Up,
  dislike: Vote.Down,
};

const voteState = (vote: Vote): VoteState =>
  vote === Vote.Up ? 'like' : vote === Vote.Down ? 'dislike' : null;

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
  const isWorld = (id: string) => recallPlace(id)?.world === true;

  return {
    ...base,
    // Mock friends until they are reimplemented in TypeScript (phase 6.9).
    friends: async () => {
      remember({ friends: FRIENDS });
      return FRIENDS;
    },
    friend: async address => FRIENDS.find(f => f.address === address) ?? null,
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
