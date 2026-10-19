import { ReactNode, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import {
  AppText,
  colors,
  Icon,
  indexFromOffset,
  LinearFill,
  PageDots,
  u,
} from '../ds';
import type { DiscoverEvent, Place } from '../discover/api';
import { t } from '../i18n';
import { EventCard, PlaceCard, Thumb } from './Cards';
import { Avatar } from './PersonRow';
import type { DiscoverFeed, Friend } from './types';

const HERO_W = u(311);
const HERO_GAP = u(8);

function Section({
  title,
  count,
  children,
}: {
  title: string;
  count?: number;
  children: ReactNode;
}) {
  return (
    <View style={styles.section}>
      <AppText variant="title" style={styles.sectionTitle}>
        {title}
        {count !== undefined && (
          <AppText variant="title" style={styles.count}>{`  ${count}`}</AppText>
        )}
      </AppText>
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        removeClippedSubviews={false}
        contentContainerStyle={styles.rail}
      >
        {children}
      </ScrollView>
    </View>
  );
}

/** EXPLORE MORE: the portrait Discover page opened from the drawer, with a back chevron to the world. */
export function DiscoverIngame({
  feed,
  friends,
  played,
  now,
  onBack,
  onPlace,
  onEvent,
}: {
  feed: DiscoverFeed;
  friends: Friend[];
  played: Place[];
  now: number;
  onBack: () => void;
  onPlace: (place: Place) => void;
  onEvent: (event: DiscoverEvent) => void;
}) {
  const insets = useSafeAreaInsets();
  const [hero, setHero] = useState(0);
  const online = friends.filter(f => f.status === 'online');

  return (
    <View style={styles.root}>
      <LinearFill
        angle={160}
        stops={[
          { offset: 0, color: '#7B27B8' },
          { offset: 1, color: colors.purple450 },
        ]}
      />
      <ScrollView
        removeClippedSubviews={false}
        showsVerticalScrollIndicator={false}
        contentContainerStyle={{
          paddingTop: insets.top + u(18),
          paddingBottom: u(32),
        }}
      >
        <View style={styles.header}>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('MENU_CLOSE')}
            hitSlop={u(8)}
            onPress={onBack}
          >
            <Icon name="back" size={22} color={colors.snow} />
          </Pressable>
          <AppText variant="heading" style={styles.headerTitle}>
            {t('MENU_DISCOVER')}
          </AppText>
          <Icon name="search" size={22} color={colors.snow} />
        </View>
        <ScrollView
          horizontal
          showsHorizontalScrollIndicator={false}
          snapToInterval={HERO_W + HERO_GAP}
          decelerationRate="fast"
          removeClippedSubviews={false}
          onScroll={e =>
            setHero(
              indexFromOffset(
                e.nativeEvent.contentOffset.x,
                HERO_W,
                HERO_GAP,
                feed.featured.length,
              ),
            )
          }
          scrollEventThrottle={32}
          contentContainerStyle={styles.hero}
        >
          {feed.featured.map(place => (
            <Pressable
              key={place.id}
              accessibilityRole="button"
              accessibilityLabel={place.title}
              onPress={() => onPlace(place)}
            >
              <Thumb
                uri={place.image}
                width={HERO_W}
                height={u(175)}
                radius={u(4)}
              />
              <AppText
                variant="heading"
                numberOfLines={2}
                style={styles.heroTitle}
              >
                {place.title}
              </AppText>
            </Pressable>
          ))}
        </ScrollView>
        {feed.featured.length > 1 && (
          <View style={styles.dots}>
            <PageDots count={feed.featured.length} index={hero} />
          </View>
        )}
        {online.length > 0 && (
          <Section title={t('DISCOVER_FRIENDS')} count={online.length}>
            {online.map(friend => (
              <View key={friend.address} style={styles.friend}>
                <Avatar uri={friend.avatar} size={u(74)} />
                <AppText
                  variant="button"
                  numberOfLines={1}
                  style={styles.friendName}
                >
                  {friend.name}
                </AppText>
                <AppText variant="caption" numberOfLines={1}>
                  {friend.place ?? ''}
                </AppText>
              </View>
            ))}
          </Section>
        )}
        {feed.events.length > 0 && (
          <Section title={t('MENU_EVENTS')}>
            {feed.events.map(event => (
              <EventCard
                key={event.id}
                event={event}
                now={now}
                featured={feed.featuredEvents.includes(event.id)}
                onPress={() => onEvent(event)}
              />
            ))}
          </Section>
        )}
        {played.length > 0 && (
          <Section title={t('MENU_LAST_PLAYED')}>
            {played.map(place => (
              <PlaceCard
                key={place.id}
                place={place}
                onPress={() => onPlace(place)}
              />
            ))}
          </Section>
        )}
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple450 },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(12),
    paddingHorizontal: u(24),
  },
  headerTitle: { fontSize: u(24), flex: 1 },
  hero: { paddingHorizontal: u(24), gap: HERO_GAP, marginTop: u(16) },
  heroTitle: {
    position: 'absolute',
    left: u(16),
    bottom: u(12),
    right: u(70),
    fontSize: u(16),
  },
  dots: { alignItems: 'center', marginTop: u(10) },
  section: { marginTop: u(20) },
  sectionTitle: { fontSize: u(17), paddingHorizontal: u(24) },
  count: { color: colors.lavender, fontSize: u(17) },
  rail: { gap: u(16), paddingHorizontal: u(24), marginTop: u(10) },
  friend: { width: u(100), alignItems: 'center', gap: u(4) },
  friendName: { fontSize: u(14) },
});
