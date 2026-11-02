import { useSyncExternalStore } from 'react';
import { Place } from './api';

const MAX = 10;
let played: Place[] = [];
const listeners = new Set<() => void>();

/** Most recent first, no duplicates; kept in memory only (the app has no storage yet). */
export function recordPlayed(place: Place) {
  played = [place, ...played.filter(entry => entry.id !== place.id)].slice(
    0,
    MAX,
  );
  listeners.forEach(listener => listener());
}

export function usePlayed(): Place[] {
  return useSyncExternalStore(
    listener => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => played,
  );
}

let searches: string[] = [];

export const loadSearchHistory = (): string[] => searches;
export const saveSearchHistory = (next: string[]) => {
  searches = next;
};
