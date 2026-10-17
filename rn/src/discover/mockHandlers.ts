import { registerMockHandler } from '../bridge/mockHandlers';
import { PLACES } from './mockData';

/** Fakes the signed requests Discover makes, for the no-engine bridge and tests. */
export function registerDiscoverMockHandlers(): void {
  const favorites = new Set<string>();
  const votes = new Map<string, string>();
  registerMockHandler(
    'places.favorite',
    (p: { place_id: string; value: boolean }) => {
      if (p.value) {
        favorites.add(p.place_id);
      } else {
        favorites.delete(p.place_id);
      }
      return {};
    },
  );
  registerMockHandler(
    'places.vote',
    (p: { place_id: string; vote: string }) => {
      votes.set(p.place_id, p.vote);
      return {};
    },
  );
  registerMockHandler('fetch.signed', (p: { url: string }) => {
    const wantsFavorites = p.url.includes('only_favorites=true');
    const rows = PLACES.filter(pl => !wantsFavorites || favorites.has(pl.id));
    return {
      status: 200,
      body: {
        ok: true,
        data: rows.map(pl => ({
          id: pl.id,
          title: pl.title,
          base_position: pl.location,
          user_favorite: favorites.has(pl.id),
          user_like: votes.get(pl.id) === 'up',
          user_dislike: votes.get(pl.id) === 'down',
        })),
      },
    };
  });
}
