import { ReactNode, useMemo, useState } from 'react';
import {
  Pressable,
  ScrollView,
  StyleSheet,
  TextInput,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, Icon, LinearFill } from '../ds';
import { ConnectionLost } from '../backpack/ConnectionLost';
import { DiscoverEvent, Place } from './api';
import { EventCard, PlaceCard } from './Cards';
import { DetailSheet } from './DetailSheet';
import { matchesQuery } from './format';
import { recordPlayed, usePlayed } from './history';
import { useDiscover } from './useDiscover';

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <View style={styles.section}>
      <AppText variant="title" style={styles.sectionTitle}>
        {title}
      </AppText>
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        contentContainerStyle={styles.rail}
      >
        {children}
      </ScrollView>
    </View>
  );
}

/** Basic Discover: featured, events, last played and most active places, with the place detail sheet. */
export function DiscoverScreen({ footer }: { footer: ReactNode }) {
  const insets = useSafeAreaInsets();
  const { state, retry } = useDiscover();
  const played = usePlayed();
  const [query, setQuery] = useState<string | null>(null);
  const [selected, setSelected] = useState<Place | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const now = useMemo(() => Date.now(), []);

  const searching = query !== null;
  const keep = (title: string) =>
    !searching || query.trim() === '' || matchesQuery(title, query);

  const jumpIn = (place: Place) => {
    recordPlayed(place);
    setSelected(null);
    setNotice(
      `Entering ${place.title} needs the world, which isn't part of this spike yet.`,
    );
    setTimeout(() => setNotice(null), 3500);
  };

  const content = () => {
    if (state.status === 'error') {
      return <ConnectionLost onRetry={retry} />;
    }
    if (state.status === 'loading') {
      return (
        <View style={styles.loading}>
          <AppText variant="caption">Loading places…</AppText>
        </View>
      );
    }
    const { featured, events, mostActive } = state.data;
    const lists = {
      featured: featured.filter(place => keep(place.title)),
      events: events.filter(event => keep(event.title)),
      played: played.filter(place => keep(place.title)),
      active: mostActive.filter(place => keep(place.title)),
    };
    const empty = Object.values(lists).every(list => list.length === 0);
    const renderPlaces = (places: Place[]) =>
      places.map(place => (
        <PlaceCard
          key={place.id}
          place={place}
          onPress={() => setSelected(place)}
        />
      ));
    return (
      <ScrollView
        showsVerticalScrollIndicator={false}
        contentContainerStyle={styles.scroll}
      >
        {empty && (
          <View style={styles.loading}>
            <AppText variant="caption">No places match “{query}”</AppText>
          </View>
        )}
        {lists.featured.length > 0 && (
          <Section title="Featured">{renderPlaces(lists.featured)}</Section>
        )}
        {lists.events.length > 0 && (
          <Section title="Events">
            {lists.events.map((event: DiscoverEvent) => (
              <EventCard
                key={event.id}
                event={event}
                now={now}
                onPress={() => setSelected(event.place)}
              />
            ))}
          </Section>
        )}
        {lists.played.length > 0 && (
          <Section title="Last Played">{renderPlaces(lists.played)}</Section>
        )}
        {lists.active.length > 0 && (
          <Section title="Most Active">{renderPlaces(lists.active)}</Section>
        )}
      </ScrollView>
    );
  };

  return (
    <View style={styles.root}>
      <LinearFill
        angle={150}
        stops={[
          { offset: 0, color: '#7A22B2' },
          { offset: 0.3, color: colors.purple450 },
          { offset: 0.7, color: colors.purple500 },
          { offset: 1, color: '#7A1AA0' },
        ]}
      />
      <View style={[styles.header, { paddingTop: insets.top + 24 }]}>
        {searching ? (
          <TextInput
            autoFocus
            value={query}
            onChangeText={setQuery}
            placeholder="Search places and events"
            placeholderTextColor={colors.silver}
            style={styles.search}
          />
        ) : (
          <AppText variant="heading" style={styles.heading}>
            Discover
          </AppText>
        )}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={searching ? 'Close search' : 'Search'}
          hitSlop={12}
          onPress={() => setQuery(searching ? null : '')}
        >
          <Icon
            name={searching ? 'close' : 'search'}
            size={26}
            strokeWidth={2.2}
          />
        </Pressable>
      </View>
      <View style={styles.body}>{content()}</View>
      {footer}
      {selected && (
        <DetailSheet
          place={selected}
          onClose={() => setSelected(null)}
          onJumpIn={jumpIn}
        />
      )}
      {notice && (
        <View
          style={[styles.notice, { bottom: 110 + insets.bottom }]}
          pointerEvents="none"
        >
          <AppText variant="body" style={styles.noticeText}>
            {notice}
          </AppText>
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: 24,
    paddingBottom: 12,
  },
  heading: { fontSize: 24 },
  search: {
    flex: 1,
    height: 40,
    marginRight: 16,
    paddingHorizontal: 14,
    borderRadius: 20,
    color: colors.snow,
    fontFamily: 'Inter-Medium',
    fontSize: 15,
    backgroundColor: colors.white10,
  },
  body: { flex: 1 },
  scroll: { paddingBottom: 24 },
  section: { marginTop: 18 },
  sectionTitle: { fontSize: 18, paddingHorizontal: 24, marginBottom: 12 },
  rail: { paddingHorizontal: 24, gap: 12 },
  loading: { alignItems: 'center', paddingTop: 48 },
  notice: {
    position: 'absolute',
    left: 24,
    right: 24,
    padding: 14,
    borderRadius: 8,
    backgroundColor: colors.shadow,
  },
  noticeText: { fontSize: 13, textAlign: 'center' },
});
