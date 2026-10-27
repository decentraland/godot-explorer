import { useEffect, useRef } from 'react';
import { ScrollView, StyleSheet } from 'react-native';
import { t } from '../i18n';
import { useApi } from './ApiContext';
import { Banner, Category } from './types';
import { Place } from './api';
import { EventCard, PlaceCard } from './Cards';
import { CategoryChips } from './CategoryChips';
import { FeatureBanners } from './FeatureBanners';
import { CompactRow, FriendCard } from './MoreCards';
import { Rail } from './Rail';
import { isOffline, RailState, useRail } from './rails';
import { useSeasonalSkin } from './seasonalSkin';
import { u } from '../ds/scale';
import { useOpeners } from './useOpeners';

export type HomeLoad = { offline: boolean; reload: () => void };

const single =
  <T,>(load: () => Promise<T[]>) =>
  async () => ({
    items: await load(),
    hasMore: false,
  });

/** The Discover rails, each loading on its own; reports when every primary rail failed. */
export function DiscoverHome({
  played,
  now,
  onJump,
  onCategory,
  onOffline,
  bannerIndex,
  startScrolled = false,
}: {
  played: Place[];
  now: number;
  onJump: (place: Place) => void;
  onCategory: (category: Category) => void;
  onOffline: (offline: boolean, reload: () => void) => void;
  bannerIndex?: number;
  startScrolled?: boolean;
}) {
  const scroll = useRef<ScrollView>(null);
  const api = useApi();
  const open = useOpeners();
  const season = useSeasonalSkin() !== undefined;
  const banners = useRail(
    single(() => api.banners(season)),
    [api, season],
  );
  const friends = useRail(
    single(() => api.friends()),
    [api],
  );
  const events = useRail(
    single(() => api.events()),
    [api],
  );
  const popular = useRail(offset => api.places('popular', offset), [api]);
  const fresh = useRail(offset => api.places('new', offset), [api]);
  const favorites = useRail(offset => api.places('favorites', offset), [api]);
  const mine = useRail(offset => api.places('mine', offset), [api]);

  const offline = isOffline([banners.state, events.state, popular.state]);
  const reload = () =>
    [banners, friends, events, popular, fresh, favorites, mine].forEach(r =>
      r.reload(),
    );
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => onOffline(offline, reload), [offline]);

  const playedState: RailState<Place> = {
    status: 'ready',
    items: played,
    hasMore: false,
  };
  const bannerList =
    banners.state.status === 'ready' ? banners.state.items : null;
  const places = (title: string, rail: typeof popular, count = false) => (
    <Rail
      title={title}
      count={count}
      state={rail.state}
      loadingMore={rail.loadingMore}
      onEnd={rail.loadMore}
      keyOf={(place, i) => `${place.id}:${i}`}
      renderItem={place => (
        <PlaceCard place={place} onPress={() => open.place(place)} />
      )}
    />
  );

  return (
    <ScrollView
      ref={scroll}
      onContentSizeChange={() =>
        startScrolled && scroll.current?.scrollToEnd({ animated: false })
      }
      removeClippedSubviews={false}
      showsVerticalScrollIndicator={false}
      contentContainerStyle={styles.scroll}
    >
      <FeatureBanners
        banners={bannerList}
        loading={banners.state.status === 'loading'}
        initialIndex={bannerIndex}
        onOpen={banner => open.place(banner.place)}
        onJump={(banner: Banner) => onJump(banner.place)}
      />
      <Rail
        title={t('DISCOVER_JUMP_BACK_IN')}
        state={playedState}
        skeleton="compact"
        keyOf={place => place.id}
        renderItem={place => (
          <CompactRow place={place} onPress={() => open.place(place)} />
        )}
      />
      {places(t('DISCOVER_POPULAR'), popular)}
      <Rail
        title={t('DISCOVER_FRIENDS')}
        count
        state={friends.state}
        keyOf={friend => friend.address}
        renderItem={friend => (
          <FriendCard friend={friend} onPress={() => open.friend(friend)} />
        )}
      />
      {places(t('DISCOVER_NEW_IN_DECENTRALAND'), fresh)}
      <CategoryChips onOpen={onCategory} />
      <Rail
        title={t('DISCOVER_EVENTS')}
        state={events.state}
        skeleton="event"
        keyOf={event => event.id}
        renderItem={event => (
          <EventCard
            event={event}
            now={now}
            onPress={() => open.event(event)}
          />
        )}
      />
      {places(t('DISCOVER_FAVORITES'), favorites)}
      {places(t('DISCOVER_MY_PLACES'), mine)}
    </ScrollView>
  );
}

const styles = StyleSheet.create({ scroll: { paddingBottom: u(24) } });
