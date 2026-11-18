import { StyleSheet, View } from 'react-native';
import {
  DISCOVER_CARD,
  DiscoverCard,
  EventBadge,
  EventTimeTag,
  u,
} from '@dcl/ui-native';
import { t } from '../i18n';
import { DiscoverEvent, Place } from './api';
import { formatRate, formatUsers, startsIn, startsInKind } from './format';
import { mockArt } from './Thumb';

const PLACE_W = DISCOVER_CARD.place.width;

export function PlaceCard({
  place,
  onPress,
  width = PLACE_W,
}: {
  place: Place;
  onPress: () => void;
  width?: number;
}) {
  return (
    <DiscoverCard
      type="place"
      title={place.title}
      image={place.image.startsWith('mock:') ? '' : place.image}
      art={mockArt(place.image)}
      users={formatUsers(place.users)}
      likeRate={formatRate(place.likeRate)}
      onPress={onPress}
      style={{ width: u(width) }}
    />
  );
}

/** LIVE + attendees, or the time pill, plus Trending (and Featured on the sheet hero). */
export function EventBadges({
  event,
  now,
  showFeatured = false,
  inline = false,
}: {
  event: DiscoverEvent;
  now: number;
  showFeatured?: boolean;
  /** Laid out by the parent (the card's tag row) instead of pinned over the image. */
  inline?: boolean;
}) {
  return (
    <View
      style={[styles.badgeRow, !inline && styles.badges]}
      pointerEvents="none"
    >
      {event.live ? (
        <EventTimeTag
          kind="live"
          label={t('DISCOVER_LIVE')}
          players={formatUsers(event.attendees)}
        />
      ) : (
        <EventTimeTag
          kind={startsInKind(event.startsAt, now)}
          label={startsIn(event.startsAt, now)}
        />
      )}
      {event.trending ? (
        <EventBadge type="trending" label={t('DISCOVER_TRENDING')} />
      ) : showFeatured && event.featured ? (
        <EventBadge type="featured" label={t('EVENT_FEATURED')} />
      ) : null}
    </View>
  );
}

export function EventCard({
  event,
  now,
  onPress,
}: {
  event: DiscoverEvent;
  now: number;
  onPress: () => void;
}) {
  return (
    <DiscoverCard
      type="event"
      title={event.title}
      image={event.image.startsWith('mock:') ? '' : event.image}
      art={mockArt(event.image)}
      byLabel={t('COMMON_BY')}
      creator={event.creator}
      tags={<EventBadges event={event} now={now} inline />}
      onPress={onPress}
    />
  );
}

const styles = StyleSheet.create({
  badgeRow: {
    flex: 1,
    flexDirection: 'row',
    justifyContent: 'space-between',
  },
  badges: { position: 'absolute', top: u(8), left: u(8), right: u(8) },
});
