import { useEffect } from 'react';
import { useWindowDimensions } from 'react-native';
import { trackClick, trackScreen } from '../analytics';
import { core } from '../core';
import type { DiscoverCarousel, DiscoverCarouselItem } from '../core/native';
import type { DiscoverEvent, Place } from './api';
import type { RailState } from './rails';
import type { Banner, Friend } from './types';

/** A Discover rail as analytics names it: the DISCOVER snapshot and the place sheet share the keys. */
export type CarouselKey =
  | 'featured'
  | 'last_visited'
  | 'most_active'
  | 'new_in_decentraland'
  | 'events'
  | 'favorites'
  | 'my_places'
  | 'friends'
  | 'search';

/** Where a sheet was opened: a rail and the card's 0-based slot, a link (no slot), or a list
 * analytics does not track (neither). */
export type CardOrigin = {
  carousel: CarouselKey | 'deeplink' | null;
  position: number | null;
};

export const DEEPLINK: CardOrigin = { carousel: 'deeplink', position: null };
export const UNTRACKED: CardOrigin = { carousel: null, position: null };

// The home only shows before entering a world: the world route replaces it.
const LOCATION = 'pre_game';

export type HomeRails = {
  banners: RailState<Banner>;
  played: Place[];
  popular: RailState<Place>;
  friends: RailState<Friend>;
  fresh: RailState<Place>;
  events: RailState<DiscoverEvent>;
  favorites: RailState<Place>;
  mine: RailState<Place>;
};

const shown = <T>(rail: RailState<T>): T[] =>
  rail.status === 'ready' ? rail.items : [];
const slot = (place: Place | null): DiscoverCarouselItem => ({
  id: place?.id ?? '',
  world: place?.world === true,
});

/** What each home rail shows, as DISCOVER's `carousels`; a rail with nothing is left out. */
export function discoverCarousels(rails: HomeRails): DiscoverCarousel[] {
  const lists: [CarouselKey, DiscoverCarouselItem[]][] = [
    ['featured', shown(rails.banners).map(banner => slot(banner.place))],
    ['last_visited', rails.played.map(slot)],
    ['most_active', shown(rails.popular).map(slot)],
    // The place the friend is in: the id their sheet logs.
    ['friends', shown(rails.friends).map(friend => slot(friend.place))],
    ['new_in_decentraland', shown(rails.fresh).map(slot)],
    [
      'events',
      shown(rails.events).map(event => ({
        id: event.id,
        world: event.place.world === true,
      })),
    ],
    ['favorites', shown(rails.favorites).map(slot)],
    ['my_places', shown(rails.mine).map(slot)],
  ];
  return lists
    .filter(([, items]) => items.length > 0)
    .map(([key, items]) => ({ key, items }));
}

/** Sends DISCOVER once the rails it waits for have settled, and again each time a rail loads after
 * that (My Places, a next page, a retry). */
export function useDiscoverSnapshot(rails: HomeRails) {
  const { banners, played, popular, friends, fresh, events, favorites, mine } =
    rails;
  useEffect(() => {
    if (
      [friends, popular, fresh, events, favorites].some(
        rail => rail.status === 'loading',
      )
    ) {
      return;
    }
    const carousels = discoverCarousels(rails);
    if (carousels.length > 0) {
      core().trackDiscover(LOCATION, carousels);
    }
    // `rails` is rebuilt every render; its parts are what changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [banners, played, popular, friends, fresh, events, favorites, mine]);
}

/** PLACE_DETAILS once the sheet shows what it was opened for (a friend sheet: their place). */
export function usePlaceDetailsView(
  shownNow: boolean,
  place: Place | null,
  origin: CardOrigin,
) {
  const { width, height } = useWindowDimensions();
  useEffect(() => {
    if (shownNow) {
      trackScreen('PLACE_DETAILS', {
        place_id: place?.id ?? 'unknown-id',
        place_title: place?.title ?? 'unknown-title',
        orientation: width > height ? 'landscape' : 'portrait',
        carousel: origin.carousel,
        position: origin.position,
      });
    }
    // once, when it first shows
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [shownNow]);
}

export function trackPlaceDetailClick(
  button: 'JUMP_IN' | 'SHARE',
  placeId: string,
  origin: CardOrigin,
) {
  trackClick(button, 'PLACE_DETAIL_CLICK', {
    place_id: placeId,
    carousel: origin.carousel,
    position: origin.position,
  });
}
