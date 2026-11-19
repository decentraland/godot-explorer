import type { ImageSourcePropType } from 'react-native';
import type { DiscoverEvent, Place } from './api';
import type { SeasonalSkin } from '../core/native';
import { getLocale, t } from '../i18n';
import type { Category, Friend } from './types';

const places = new Map<string, Place>();
const events = new Map<string, DiscoverEvent>();
const friends = new Map<string, Friend>();

/** Items any list has shown, so a sheet opened from a card resolves without another fetch. */
export function remember(items: {
  places?: Place[];
  events?: DiscoverEvent[];
  friends?: Friend[];
}) {
  items.places?.forEach(place => places.set(place.id, place));
  items.events?.forEach(event => {
    events.set(event.id, event);
    places.set(event.place.id, event.place);
  });
  items.friends?.forEach(friend => friends.set(friend.address, friend));
}

export const recallPlace = (id: string) => places.get(id);
export const recallEvent = (id: string) => events.get(id);
export const recallFriend = (address: string) => friends.get(address);

export function forgetAll() {
  places.clear();
  events.clear();
  friends.clear();
}

const FIXED = [
  {
    id: 'game',
    name: 'CATEGORY_GAME',
    art: require('../assets/discover/category-game.png'),
  },
  {
    id: 'social',
    name: 'CATEGORY_SOCIAL',
    art: require('../assets/discover/category-social.png'),
  },
  {
    id: 'music',
    name: 'CATEGORY_MUSIC',
    art: require('../assets/discover/category-music.png'),
  },
  {
    id: 'art',
    name: 'CATEGORY_ART',
    art: require('../assets/discover/category-art.png'),
  },
] as const;

export const SEASONAL = 'seasonal';

/** The Discover category row: the season's tile first while one is active, then the fixed four.
 * `seasonArt` replaces the remote tile image (gallery). */
export function categories(
  skin?: SeasonalSkin,
  seasonArt?: ImageSourcePropType,
): Category[] {
  const fixed: Category[] = FIXED.map(c => ({
    id: c.id,
    tile: c.id,
    name: t(c.name),
    art: c.art,
  }));
  if (!skin) {
    return fixed;
  }
  const name = skin.names.get(getLocale()) ?? skin.names.get('en') ?? skin.key;
  return [
    {
      id: SEASONAL,
      tile: SEASONAL,
      name,
      art: seasonArt ?? { uri: skin.tileUrl },
      season: skin.key,
    },
    ...fixed,
  ];
}
