import type { Place } from './api';
import type { SearchResults, Suggestion } from './types';

export const SEARCH_DEBOUNCE_MS = 400;
export const SUGGEST_MIN_CHARS = 2;
const MAX_HISTORY_SHOWN = 4;
const MAX_HISTORY = 10;

export function parseCoordinates(
  query: string,
): { x: number; y: number } | null {
  const m = query.match(/^\s*(-?\d+)\s*,\s*(-?\d+)\s*$/);
  return m ? { x: Number(m[1]), y: Number(m[2]) } : null;
}

/** Strips the punctuation the places search chokes on. */
export const normalizeKeyword = (query: string): string =>
  query
    .replace(/[.*!?¡¿\-_[\]<>'"%&\\/,;:]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

export function historyMatches(history: string[], query: string): string[] {
  const needle = query.trim().toLowerCase();
  return history
    .filter(item => item.toLowerCase().includes(needle))
    .slice(0, MAX_HISTORY_SHOWN);
}

export type SearchPhase =
  | 'idle'
  | 'suggest'
  | 'loading'
  | 'results'
  | 'empty'
  | 'error';

export type SearchState = {
  query: string;
  phase: SearchPhase;
  suggestions: Suggestion[];
  results: SearchResults | null;
  history: string[];
};

export type SearchAction =
  | { type: 'change'; text: string }
  | { type: 'suggestions'; items: Suggestion[] }
  | { type: 'submit'; text: string }
  | { type: 'results'; results: SearchResults }
  | { type: 'fail' }
  | { type: 'clearHistory' };

export const initialSearch = (
  history: string[],
  over: Partial<SearchState> = {},
): SearchState => ({
  query: '',
  phase: 'idle',
  suggestions: [],
  results: null,
  history,
  ...over,
});

export const hasResults = (r: SearchResults, lastPlayed: Place[] = []) =>
  r.featured.length +
    r.events.length +
    r.mostActive.length +
    r.favorites.length +
    lastPlayed.length >
  0;

export function searchReducer(
  state: SearchState,
  action: SearchAction,
): SearchState {
  switch (action.type) {
    case 'change':
      return {
        ...state,
        query: action.text,
        phase: action.text === '' ? 'idle' : 'suggest',
        suggestions: action.text === '' ? [] : state.suggestions,
        results: null,
      };
    case 'suggestions':
      return { ...state, suggestions: action.items };
    case 'submit':
      return { ...state, query: action.text, phase: 'loading', results: null };
    case 'results': {
      if (!hasResults(action.results)) {
        return { ...state, phase: 'empty', results: action.results };
      }
      const history = [
        state.query,
        ...state.history.filter(item => item !== state.query),
      ].slice(0, MAX_HISTORY);
      return { ...state, phase: 'results', results: action.results, history };
    }
    case 'fail':
      return { ...state, phase: 'error' };
    case 'clearHistory':
      return { ...state, history: [] };
  }
}
