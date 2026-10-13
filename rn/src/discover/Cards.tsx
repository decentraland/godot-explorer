import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, LinearFill } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { DiscoverEvent, Place } from './api';
import { formatRate, formatUsers, startsIn } from './format';
import { Thumb } from './Thumb';

export const PLACE_W = 147;
export const EVENT_W = 203;
const PLACE_H = 78;
const EVENT_H = 120;

export function Stats({ users, rate }: { users: number; rate: number | null }) {
  return (
    <View style={styles.stats}>
      <Icon name="user" size={13} color={colors.gray} strokeWidth={2} />
      <AppText variant="caption" style={styles.stat}>
        {formatUsers(users)}
      </AppText>
      <Icon name="like" size={13} color={colors.gray} strokeWidth={2} />
      <AppText variant="caption" style={styles.stat}>
        {formatRate(rate)}
      </AppText>
    </View>
  );
}

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
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={place.title}
      onPress={onPress}
      style={({ pressed }) => [{ width: u(width) }, pressed && styles.pressed]}
    >
      <Thumb uri={place.image} width={u(width)} height={u(PLACE_H)} />
      <AppText variant="button" numberOfLines={1} style={styles.title}>
        {place.title}
      </AppText>
      <Stats users={place.users} rate={place.likeRate} />
    </Pressable>
  );
}

/** LIVE + attendees, or the time pill, plus Trending (and Featured on the sheet hero). */
export function EventBadges({
  event,
  now,
  showFeatured = false,
}: {
  event: DiscoverEvent;
  now: number;
  showFeatured?: boolean;
}) {
  return (
    <View style={styles.badges} pointerEvents="none">
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
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={event.title}
      onPress={onPress}
      style={({ pressed }) => [
        { width: u(EVENT_W) },
        pressed && styles.pressed,
      ]}
    >
      <Thumb uri={event.image} width={u(EVENT_W)} height={u(EVENT_H)} />
      <EventBadges event={event} now={now} />
      <AppText variant="button" numberOfLines={1} style={styles.eventTitle}>
        {event.title}
      </AppText>
      <AppText variant="body" numberOfLines={1} style={styles.by}>
        {t('COMMON_BY')} {event.creator}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  title: { marginTop: u(5), fontSize: u(13) },
  eventTitle: { marginTop: u(8), fontSize: u(14) },
  by: { marginTop: u(2), fontSize: u(13), color: colors.gray },
  stats: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: u(2.5),
    gap: u(4),
  },
  stat: {
    fontSize: u(13),
    marginRight: u(8),
    marginLeft: u(2),
    color: colors.pearl,
  },
  badges: {
    position: 'absolute',
    top: u(8),
    left: u(8),
    right: u(8),
    flexDirection: 'row',
    justifyContent: 'space-between',
  },
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
  pressed: { opacity: 0.85 },
});
