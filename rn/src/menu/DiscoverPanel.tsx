import { ReactNode, useEffect, useRef, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, Skeleton, u } from '../ds';
import type { DiscoverEvent, Place } from '../discover/api';
import { t } from '../i18n';
import { EventCard, PLACE_W, PlaceCard } from './Cards';
import { KebabAction, KebabMenu } from './KebabMenu';
import type { Loaded } from './useLoad';
import type { DiscoverFeed, SceneInfo } from './types';

export type DiscoverPanelProps = {
  scene: SceneInfo | null;
  feed: Loaded<DiscoverFeed>;
  now: number;
  onPlace: (place: Place) => void;
  onEvent: (event: DiscoverEvent) => void;
  onExploreMore: () => void;
  onKebab: (action: KebabAction) => void;
  onRetry: () => void;
  /** Gallery states: kebab already open, list scrolled to the end. */
  initialKebabOpen?: boolean;
  scrolledToEnd?: boolean;
};

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <View style={styles.section}>
      <AppText variant="title" style={styles.sectionTitle}>
        {title}
      </AppText>
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        removeClippedSubviews={false}
        nestedScrollEnabled
        contentContainerStyle={styles.rail}
      >
        {children}
      </ScrollView>
    </View>
  );
}

function Loading() {
  return (
    <View style={styles.section}>
      <Skeleton width={u(90)} height={u(16)} radius={u(4)} />
      <View style={styles.rail}>
        <Skeleton width={PLACE_W} height={u(78)} radius={u(4)} />
        <Skeleton width={PLACE_W} height={u(78)} radius={u(4)} />
      </View>
    </View>
  );
}

export function DiscoverPanel({
  scene,
  feed,
  now,
  onPlace,
  onEvent,
  onExploreMore,
  onKebab,
  onRetry,
  initialKebabOpen = false,
  scrolledToEnd = false,
}: DiscoverPanelProps) {
  const [kebabOpen, setKebabOpen] = useState(initialKebabOpen);
  const list = useRef<ScrollView>(null);
  useEffect(() => {
    if (scrolledToEnd) {
      const id = setTimeout(
        () => list.current?.scrollToEnd({ animated: false }),
        50,
      );
      return () => clearTimeout(id);
    }
  }, [scrolledToEnd]);

  const body = () => {
    if (feed.status === 'loading') {
      return <Loading />;
    }
    if (feed.status === 'error') {
      return (
        <View style={styles.error}>
          <AppText variant="body" style={styles.errorText}>
            {t('MENU_PLACE_ERROR')}
          </AppText>
          <Pressable accessibilityRole="button" onPress={onRetry}>
            <AppText variant="button" style={styles.retry}>
              {t('MENU_RETRY')}
            </AppText>
          </Pressable>
        </View>
      );
    }
    const { featured, events, featuredEvents } = feed.data;
    return (
      <>
        {featured.length > 0 && (
          <Section title={t('MENU_FEATURED')}>
            {featured.map(place => (
              <PlaceCard
                key={place.id}
                place={place}
                onPress={() => onPlace(place)}
              />
            ))}
          </Section>
        )}
        {events.length > 0 && (
          <Section title={t('MENU_EVENTS')}>
            {events.map(event => (
              <EventCard
                key={event.id}
                event={event}
                now={now}
                featured={featuredEvents.includes(event.id)}
                onPress={() => onEvent(event)}
              />
            ))}
          </Section>
        )}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('MENU_EXPLORE_MORE').toUpperCase()}
          onPress={onExploreMore}
          style={({ pressed }) => [styles.explore, pressed && styles.pressed]}
        >
          <AppText variant="button" style={styles.exploreText}>
            {t('MENU_EXPLORE_MORE').toUpperCase()}
          </AppText>
        </Pressable>
      </>
    );
  };

  return (
    <View style={styles.root}>
      <ScrollView
        ref={list}
        showsVerticalScrollIndicator={false}
        removeClippedSubviews={false}
        contentContainerStyle={styles.content}
      >
        <View style={styles.header}>
          <View style={styles.titles}>
            <AppText variant="heading" numberOfLines={1} style={styles.title}>
              {scene?.title ?? t('MENU_UNKNOWN_SCENE')}
            </AppText>
            {!!scene?.creator && (
              <AppText variant="body" style={styles.creator}>
                {t('MENU_BY_CREATOR', { creator: scene.creator })}
              </AppText>
            )}
          </View>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('MENU_MORE_OPTIONS')}
            hitSlop={u(8)}
            onPress={() => setKebabOpen(open => !open)}
            style={styles.kebab}
          >
            <Icon
              name="kebab"
              size={24}
              color={colors.lavender}
              strokeWidth={4}
            />
          </Pressable>
        </View>
        {body()}
      </ScrollView>
      {kebabOpen && (
        <>
          <Pressable
            accessibilityLabel={t('MENU_CLOSE')}
            style={StyleSheet.absoluteFill}
            onPress={() => setKebabOpen(false)}
          />
          <View style={styles.popover}>
            <KebabMenu
              onSelect={action => {
                setKebabOpen(false);
                onKebab(action);
              }}
            />
          </View>
        </>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1 },
  content: { paddingLeft: u(24.5), paddingTop: u(12), paddingBottom: u(24) },
  header: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    paddingRight: u(24),
  },
  titles: { flex: 1, paddingRight: u(12) },
  title: { fontSize: u(20), lineHeight: u(26) },
  creator: { fontSize: u(13), color: colors.white70, marginTop: u(2) },
  kebab: {
    width: u(32),
    height: u(32),
    alignItems: 'center',
    justifyContent: 'center',
  },
  section: { marginTop: u(14) },
  sectionTitle: { fontSize: u(16), color: colors.snow },
  rail: { gap: u(16), marginTop: u(10), paddingRight: u(24) },
  explore: {
    width: u(262),
    height: u(37),
    marginTop: u(26),
    alignSelf: 'flex-start',
    borderRadius: u(7),
    borderWidth: 1,
    borderColor: colors.snow,
    backgroundColor: colors.purple550,
    alignItems: 'center',
    justifyContent: 'center',
  },
  pressed: { opacity: 0.8 },
  exploreText: { fontFamily: 'Inter-Bold', fontSize: u(14) },
  error: {
    marginTop: u(40),
    alignItems: 'center',
    gap: u(10),
    paddingRight: u(24),
  },
  errorText: { fontSize: u(13), color: colors.white70 },
  retry: { color: colors.ruby, fontSize: u(14) },
  popover: { position: 'absolute', top: u(54), right: u(24) },
});
