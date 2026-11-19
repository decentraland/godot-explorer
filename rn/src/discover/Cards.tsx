import { StyleSheet, View } from 'react-native';
import {
  AppText,
  colors,
  DISCOVER_CARD,
  DiscoverCard,
  Icon,
  LinearFill,
  u,
} from '@dcl/ui-native';
import { t } from '../i18n';
import { DiscoverEvent, Place } from './api';
import { formatRate, formatUsers, startsIn } from './format';
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
        <View style={styles.row}>
          <View style={[styles.badge, styles.live]}>
            <AppText variant="caption" style={styles.badgeText}>
              {t('DISCOVER_LIVE')}
            </AppText>
          </View>
          <View style={[styles.badge, styles.users]}>
            <View style={styles.dot} />
            <Icon name="user" size={10} color={colors.snow} strokeWidth={2} />
            <AppText variant="caption" style={styles.usersText}>
              {formatUsers(event.attendees)}
            </AppText>
          </View>
        </View>
      ) : (
        <View style={[styles.badge, styles.soon]}>
          <AppText
            variant="caption"
            style={[styles.badgeText, styles.soonText]}
          >
            {startsIn(event.startsAt, now)}
          </AppText>
        </View>
      )}
      {event.trending ? (
        <View style={[styles.badge, styles.trending]}>
          <AppText variant="caption" style={styles.badgeText}>
            {t('DISCOVER_TRENDING')}
          </AppText>
        </View>
      ) : showFeatured && event.featured ? (
        <View style={[styles.badge, styles.featured]}>
          <LinearFill
            angle={90}
            stops={[
              { offset: 0, color: colors.ruby },
              { offset: 1, color: colors.mythic },
            ]}
          />
          <AppText variant="caption" style={styles.badgeText}>
            {t('EVENT_FEATURED')}
          </AppText>
        </View>
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
  row: { flexDirection: 'row', gap: u(4) },
  badge: {
    height: u(16),
    paddingHorizontal: u(5),
    borderRadius: u(2),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(3),
    overflow: 'hidden',
  },
  badgeText: {
    fontFamily: 'Inter-SemiBold',
    color: colors.snow,
    fontSize: u(9.5),
    textTransform: 'none',
  },
  live: { backgroundColor: colors.ruby },
  users: { backgroundColor: 'rgba(0,0,0,0.65)' },
  usersText: { color: colors.snow, fontSize: u(9.5) },
  dot: {
    width: u(5),
    height: u(5),
    borderRadius: u(3),
    backgroundColor: '#2ECC71',
  },
  trending: { backgroundColor: colors.teal },
  featured: { backgroundColor: colors.mythic },
  soon: { backgroundColor: colors.snow },
  soonText: { color: '#161518' },
});
