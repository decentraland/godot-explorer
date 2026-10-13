import { useCallback, useEffect, useReducer, useRef } from 'react';
import { useApi } from './ApiContext';
import { loadSearchHistory, saveSearchHistory } from './history';
import {
  historyMatches,
  initialSearch,
  normalizeKeyword,
  SEARCH_DEBOUNCE_MS,
  SearchState,
  searchReducer,
  SUGGEST_MIN_CHARS,
} from './search';
import type { Suggestion } from './types';

const MAX_SUGGESTIONS = 8;
const mergeSuggestions = (local: Suggestion[], remote: Suggestion[]) => {
  const seen = new Set(local.map(item => item.text));
  return [...local, ...remote.filter(item => !seen.has(item.text))].slice(
    0,
    MAX_SUGGESTIONS,
  );
};

/** Search state machine over the api: debounced suggestions, submit, stale-response guard. */
export function useSearch(initial?: Partial<SearchState>) {
  const api = useApi();
  const [state, dispatch] = useReducer(searchReducer, undefined, () =>
    initialSearch(loadSearchHistory(), initial),
  );
  const request = useRef(0);

  useEffect(() => saveSearchHistory(state.history), [state.history]);

  useEffect(() => {
    if (state.phase !== 'suggest') {
      return undefined;
    }
    const local: Suggestion[] = historyMatches(state.history, state.query).map(
      text => ({
        kind: 'history',
        text,
      }),
    );
    if (state.query.trim().length < SUGGEST_MIN_CHARS) {
      dispatch({ type: 'suggestions', items: local });
      return undefined;
    }
    const id = ++request.current;
    const timer = setTimeout(() => {
      api
        .suggest(normalizeKeyword(state.query))
        .then(
          items =>
            id === request.current &&
            dispatch({
              type: 'suggestions',
              items: mergeSuggestions(local, items),
            }),
        )
        .catch(
          () =>
            id === request.current &&
            dispatch({ type: 'suggestions', items: local }),
        );
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
    // history only seeds the local rows; re-querying on every history write would loop
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.phase, state.query, api]);

  const submit = useCallback(
    (text: string) => {
      const query = text.trim();
      if (!query) {
        return;
      }
      const id = ++request.current;
      dispatch({ type: 'submit', text: query });
      api
        .search(query)
        .then(
          results =>
            id === request.current && dispatch({ type: 'results', results }),
        )
        .catch(() => id === request.current && dispatch({ type: 'fail' }));
    },
    [api],
  );

  const startPhase = initial?.phase;
  const startQuery = initial?.query;
  useEffect(() => {
    if (startPhase === 'loading' && startQuery) {
      submit(startQuery);
    }
    // once, to run a search the screen was opened on
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const change = useCallback((text: string) => {
    request.current += 1;
    dispatch({ type: 'change', text });
  }, []);
  const clearHistory = useCallback(
    () => dispatch({ type: 'clearHistory' }),
    [],
  );
  return { state, change, submit, clearHistory };
}
