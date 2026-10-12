import { Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon } from '../ds';
import { DiscoverEvent, Place } from './api';
import { formatRate, formatUsers, startsIn } from './format';

const PLACE_W = 146;
const EVENT_W = 203;

function Thumb({
  uri,
  width,
  height,
}: {
  uri: string;
  width: number;
  height: number;
}) {
  return (
    <View style={[styles.thumb, { width, height }]}>
      {uri ? (
        <Image source={{ uri }} style={styles.fill} resizeMode="cover" />
      ) : null}
    </View>
  );
}

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
}: {
  place: Place;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={place.title}
      onPress={onPress}
      style={({ pressed }) => [{ width: PLACE_W }, pressed && styles.pressed]}
    >
      <Thumb uri={place.image} width={PLACE_W} height={78} />
      <AppText variant="button" numberOfLines={1} style={styles.title}>
        {place.title}
      </AppText>
      <Stats users={place.users} rate={place.likeRate} />
    </Pressable>
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
      style={({ pressed }) => [{ width: EVENT_W }, pressed && styles.pressed]}
    >
      <Thumb uri={event.image} width={EVENT_W} height={120} />
      <View style={styles.badges} pointerEvents="none">
        {event.live ? (
          <View style={styles.row}>
            <View style={[styles.badge, styles.live]}>
              <AppText variant="caption" style={styles.badgeText}>
                LIVE
              </AppText>
            </View>
            <View style={[styles.badge, styles.users]}>
              <View style={styles.dot} />
              <Icon name="user" size={11} color={colors.snow} strokeWidth={2} />
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
              Trending
            </AppText>
          </View>
        ) : null}
      </View>
      <AppText variant="button" numberOfLines={1} style={styles.eventTitle}>
        {event.title}
      </AppText>
      <AppText variant="body" numberOfLines={1} style={styles.by}>
        by {event.creator}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  thumb: {
    borderRadius: 4,
    overflow: 'hidden',
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.35)',
    backgroundColor: colors.purple450,
  },
  fill: { width: '100%', height: '100%' },
  title: { marginTop: 8, fontSize: 13 },
  eventTitle: { marginTop: 8, fontSize: 15 },
  by: { marginTop: 4, fontSize: 13, color: colors.gray },
  stats: { flexDirection: 'row', alignItems: 'center', marginTop: 4, gap: 4 },
  stat: { fontSize: 12, marginRight: 8, color: colors.pearl },
  badges: {
    position: 'absolute',
    top: 8,
    left: 8,
    right: 8,
    flexDirection: 'row',
    justifyContent: 'space-between',
  },
  row: { flexDirection: 'row', gap: 4 },
  badge: {
    height: 20,
    paddingHorizontal: 6,
    borderRadius: 3,
    flexDirection: 'row',
    alignItems: 'center',
    gap: 3,
  },
  badgeText: {
    fontFamily: 'Inter-SemiBold',
    color: colors.snow,
    fontSize: 11,
    textTransform: 'none',
  },
  live: { backgroundColor: colors.ruby },
  users: { backgroundColor: 'rgba(0,0,0,0.65)' },
  usersText: { color: colors.snow, fontSize: 11 },
  dot: { width: 5, height: 5, borderRadius: 3, backgroundColor: '#2ECC71' },
  trending: { backgroundColor: '#2ECC8A' },
  soon: { backgroundColor: colors.snow },
  soonText: { color: colors.ruby },
  pressed: { opacity: 0.85 },
});
