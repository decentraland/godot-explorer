import type { DiscoverEvent, Place } from './api';
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

export const CATEGORIES: Category[] = [
  {
    id: 'social',
    label: 'CATEGORY_SOCIAL',
    title: 'CATEGORY_SOCIAL_TITLE',
    emoji: '🧑‍🤝‍🧑',
  },
  {
    id: 'music',
    label: 'CATEGORY_MUSIC',
    title: 'CATEGORY_MUSIC_TITLE',
    emoji: '🎵',
  },
  {
    id: 'game',
    label: 'CATEGORY_GAME',
    title: 'CATEGORY_GAME_TITLE',
    emoji: '🎮',
  },
  {
    id: 'art',
    label: 'CATEGORY_ART',
    title: 'CATEGORY_ART_TITLE',
    emoji: '🎨',
  },
  {
    id: 'fashion',
    label: 'CATEGORY_FASHION',
    title: 'CATEGORY_FASHION_TITLE',
    emoji: '👗',
  },
  {
    id: 'business',
    label: 'CATEGORY_BUSINESS',
    title: 'CATEGORY_BUSINESS_TITLE',
    emoji: '🏢',
  },
];
