import { useEffect, useState } from 'react';
import { FlatList, StyleSheet, View } from 'react-native';
import { SearchField } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { Place } from './api';
import { useApi } from './ApiContext';
import { PlaceCard } from './Cards';
import { DiscoverHeader } from './DiscoverHeader';
import { MessageState } from './MessageState';
import { PlaceSkeleton } from './MoreCards';
import { useRail } from './rails';
import { SEARCH_DEBOUNCE_MS } from './search';
import type { Category } from './types';
import { useOpeners } from './useOpeners';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

/** Two-column grid of a category's places with infinite scroll and an inline search. */
export function CategoryScreen({
  category,
  onBack,
  startSearching = false,
}: {
  category: Category;
  onBack: () => void;
  startSearching?: boolean;
}) {
  const api = useApi();
  const open = useOpeners();
  const insets = useSafeAreaInsets();
  const [searching, setSearching] = useState(startSearching);
  const [text, setText] = useState('');
  const [query, setQuery] = useState('');

  useEffect(() => {
    const timer = setTimeout(() => setQuery(text.trim()), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [text]);

  const { state, loadMore, loadingMore, reload } = useRail(
    offset => api.categoryPlaces(category.id, offset, query || undefined),
    [api, category.id, query],
  );

  const closeSearch = () => {
    setSearching(false);
    setText('');
    setQuery('');
  };

  return (
    <View style={styles.root}>
      {searching ? (
        <View style={[styles.search, { paddingTop: insets.top + u(6) }]}>
          <SearchField
            framed
            autoFocus={!startSearching}
            value={text}
            onChangeText={setText}
            onBack={closeSearch}
            placeholder={t('DISCOVER_SEARCH')}
          />
        </View>
      ) : (
        <DiscoverHeader
          title={t(category.title)}
          onBack={onBack}
          onSearch={() => setSearching(true)}
        />
      )}
      {state.status === 'error' ? (
        <MessageState
          glyph="search"
          title={t('DISCOVER_SOMETHING_WENT_WRONG')}
          body={t('DISCOVER_WE_COULDN_T_LOAD_THE_RESULTS')}
          onRetry={reload}
        />
      ) : state.status === 'ready' && state.items.length === 0 ? (
        <MessageState
          glyph="search"
          title={t('DISCOVER_NO_RESULT_FOUND')}
          body={t('DISCOVER_TRY_SEARCHING_WITH_DIFFERENT_KEYWORDS')}
        />
      ) : (
        <FlatList<Place | null>
          data={
            state.status === 'ready'
              ? [
                  ...state.items,
                  ...(loadingMore || state.hasMore ? [null, null] : []),
                ]
              : Array.from({ length: 6 }, () => null)
          }
          numColumns={2}
          keyExtractor={(item, i) => `${item?.id ?? 'skeleton'}:${i}`}
          columnWrapperStyle={styles.columns}
          contentContainerStyle={styles.grid}
          keyboardShouldPersistTaps="handled"
          showsVerticalScrollIndicator={false}
          onEndReached={loadMore}
          onEndReachedThreshold={0.5}
          renderItem={({ item }) =>
            item ? (
              <PlaceCard place={item} onPress={() => open.place(item)} />
            ) : (
              <PlaceSkeleton />
            )
          }
        />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1 },
  search: { paddingHorizontal: u(24), paddingBottom: u(10) },
  grid: {
    paddingHorizontal: u(24),
    paddingTop: u(14),
    paddingBottom: u(24),
    gap: u(18),
  },
  columns: { gap: u(18) },
});
