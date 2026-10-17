import { ReactNode, useMemo } from 'react';
import {
  Keyboard,
  Pressable,
  ScrollView,
  StyleSheet,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, Icon, SearchField } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { DiscoverEvent, Place } from './api';
import { EventCard, PlaceCard } from './Cards';
import { matchesQuery } from './format';
import { MessageState } from './MessageState';
import { Rail } from './Rail';
import type { SearchState } from './search';
import type { Suggestion } from './types';
import { useSearch } from './useSearch';

function Row({
  icon,
  text,
  onPress,
}: {
  icon: 'search' | 'clock';
  text: string;
  onPress: () => void;
}) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} style={styles.row}>
      <Icon name={icon} size={15} color={colors.snow} strokeWidth={2} />
      <AppText variant="body" style={styles.rowText}>
        {text}
      </AppText>
    </Pressable>
  );
}

const ready = <T,>(items: T[]) => ({
  status: 'ready' as const,
  items,
  hasMore: false,
});

/**
 * Search mode: field header, then recents / suggestions / skeleton / result rails / message by phase.
 * `idleContent` stays visible while the field is empty and there is no history.
 */
export function SearchView({
  initial,
  onBack,
  onPlace,
  onEvent,
  lastPlayed,
  idleContent,
  now,
}: {
  initial?: Partial<SearchState>;
  onBack: () => void;
  onPlace: (place: Place) => void;
  onEvent: (event: DiscoverEvent) => void;
  lastPlayed: Place[];
  idleContent?: ReactNode;
  now: number;
}) {
  const insets = useSafeAreaInsets();
  const { state, change, submit, clearHistory } = useSearch(initial);
  const { phase, query, results, history, suggestions } = state;
  const played = useMemo(
    () => lastPlayed.filter(place => matchesQuery(place.title, query)),
    [lastPlayed, query],
  );

  const pick = (item: Suggestion) => {
    Keyboard.dismiss();
    submit(item.text);
  };
  const places = (title: string, list: Place[]) => (
    <Rail
      title={title}
      state={ready(list)}
      keyOf={place => place.id}
      renderItem={place => (
        <PlaceCard place={place} onPress={() => onPlace(place)} />
      )}
    />
  );

  const body = () => {
    switch (phase) {
      case 'idle':
        return history.length > 0 ? (
          <View>
            <View style={styles.recentHead}>
              <AppText variant="title" style={styles.recentTitle}>
                {t('DISCOVER_RECENT_SEARCHES')}
              </AppText>
              <Pressable accessibilityRole="button" onPress={clearHistory}>
                <AppText variant="caption" style={styles.clean}>
                  {t('DISCOVER_CLEAN_ALL')}
                </AppText>
              </Pressable>
            </View>
            {history.slice(0, 4).map(text => (
              <Row
                key={text}
                icon="clock"
                text={text}
                onPress={() => pick({ kind: 'history', text })}
              />
            ))}
          </View>
        ) : (
          idleContent
        );
      case 'suggest':
        return (
          <View>
            {suggestions.map(item => (
              <Row
                key={`${item.kind}:${item.text}`}
                icon={item.kind === 'history' ? 'clock' : 'search'}
                text={item.text}
                onPress={() => pick(item)}
              />
            ))}
          </View>
        );
      case 'loading':
        return (
          <>
            {(
              [
                'DISCOVER_FEATURED',
                'DISCOVER_EVENTS',
                'DISCOVER_LAST_VISITED',
              ] as const
            ).map(key => (
              <Rail
                key={key}
                title={t(key)}
                state={{ status: 'loading' }}
                skeleton={key === 'DISCOVER_EVENTS' ? 'event' : 'place'}
                keyOf={() => key}
                renderItem={() => null}
              />
            ))}
          </>
        );
      case 'results':
        return results ? (
          <>
            {places(t('DISCOVER_FEATURED'), results.featured)}
            <Rail
              title={t('DISCOVER_EVENTS')}
              state={ready(results.events)}
              keyOf={event => event.id}
              skeleton="event"
              renderItem={event => (
                <EventCard
                  event={event}
                  now={now}
                  onPress={() => onEvent(event)}
                />
              )}
            />
            {places(t('DISCOVER_LAST_VISITED'), played)}
            {places(t('DISCOVER_MOST_ACTIVES'), results.mostActive)}
            {places(t('DISCOVER_FAVORITES'), results.favorites)}
          </>
        ) : null;
      case 'empty':
        return (
          <MessageState
            glyph="search"
            title={t('DISCOVER_NO_RESULT_FOUND')}
            body={t('DISCOVER_TRY_SEARCHING_WITH_DIFFERENT_KEYWORDS')}
          />
        );
      case 'error':
        return (
          <MessageState
            glyph="search"
            title={t('DISCOVER_SOMETHING_WENT_WRONG')}
            body={t('DISCOVER_WE_COULDN_T_LOAD_THE_RESULTS')}
            onRetry={() => submit(query)}
          />
        );
    }
  };

  return (
    <View style={styles.root}>
      <View
        style={[
          styles.header,
          { paddingTop: insets.top + u(6) },
          (phase === 'loading' || phase === 'results') && styles.headerRails,
        ]}
      >
        <SearchField
          framed
          autoFocus={
            initial?.phase === undefined ||
            initial.phase === 'idle' ||
            initial.phase === 'suggest'
          }
          value={query}
          onChangeText={change}
          onSubmit={submit}
          onBack={onBack}
          placeholder={t('DISCOVER_SEARCH')}
        />
      </View>
      <ScrollView
        keyboardShouldPersistTaps="handled"
        showsVerticalScrollIndicator={false}
        contentContainerStyle={styles.scroll}
      >
        {body()}
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1 },
  header: { paddingHorizontal: u(24), paddingBottom: u(10) },
  headerRails: { paddingBottom: u(4) },
  scroll: { paddingBottom: u(24), flexGrow: 1 },
  recentHead: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: u(24),
    marginBottom: u(8),
  },
  recentTitle: { fontSize: u(15), color: colors.lavender70 },
  clean: {
    fontSize: u(12),
    color: colors.lavender,
    textDecorationLine: 'underline',
  },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(10),
    marginHorizontal: u(24),
    height: u(60),
    borderBottomWidth: 1,
    borderBottomColor: colors.white70,
  },
  rowText: { fontSize: u(15), color: colors.snow },
});
