import { Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, u } from '../ds';
import type { DiscoverEvent, Place } from '../discover/api';
import { formatRate, formatUsers, startsIn } from '../discover/format';
import { t } from '../i18n';
import { ArtFill } from './ArtFill';
import { FeaturedTag, LiveTag, SoonTag, TrendingTag, UsersTag } from './Tags';

export const PLACE_W = u(147);
export const EVENT_W = u(203);

export function Thumb({
  uri,
  width,
  height,
  radius = u(4),
  seed,
}: {
  uri: string;
  seed?: string;
  width: number;
  height: number;
  radius?: number;
}) {
  return (
    <View style={[styles.thumb, { width, height, borderRadius: radius }]}>
      {uri ? (
        <Image source={{ uri }} style={styles.fill} resizeMode="cover" />
      ) : (
        <ArtFill seed={seed} />
      )}
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
      <Thumb
        uri={place.image}
        seed={place.title}
        width={PLACE_W}
        height={u(78)}
      />
      <AppText variant="button" numberOfLines={1} style={styles.placeTitle}>
        {place.title}
      </AppText>
      <View style={styles.stats}>
        <Icon name="user" size={13} color={colors.white70} strokeWidth={2} />
        <AppText variant="body" style={styles.stat}>
          {formatUsers(place.users)}
        </AppText>
        <Icon name="like" size={13} color={colors.white70} strokeWidth={2} />
        <AppText variant="body" style={styles.stat}>
          {formatRate(place.likeRate)}
        </AppText>
      </View>
    </Pressable>
  );
}

/** The tag row shared by the event card and the event modal image. */
export function EventTags({
  event,
  featured,
  now,
}: {
  event: DiscoverEvent;
  featured: boolean;
  now: number;
}) {
  return (
    <View style={styles.tags} pointerEvents="none">
      <View style={styles.tagGroup}>
        {event.live ? (
          <>
            <LiveTag />
            <UsersTag count={event.attendees} />
          </>
        ) : (
          <SoonTag label={startsIn(event.startsAt, now)} />
        )}
      </View>
      {featured ? (
        <FeaturedTag />
      ) : event.live && event.trending ? (
        <TrendingTag />
      ) : null}
    </View>
  );
}

export function EventCard({
  event,
  featured,
  now,
  onPress,
}: {
  event: DiscoverEvent;
  featured: boolean;
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
      <View>
        <Thumb
          uri={event.image}
          seed={event.title}
          width={EVENT_W}
          height={u(120)}
        />
        <EventTags event={event} featured={featured} now={now} />
      </View>
      <AppText variant="button" numberOfLines={2} style={styles.eventTitle}>
        {event.title}
      </AppText>
      <AppText variant="body" numberOfLines={1} style={styles.by}>
        {t('MENU_BY_CREATOR', { creator: event.creator })}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  thumb: {
    overflow: 'hidden',
    borderWidth: 0.5,
    borderColor: 'rgba(232,185,255,0.5)',
    backgroundColor: colors.purple450,
  },
  fill: { width: '100%', height: '100%' },
  pressed: { opacity: 0.8 },
  placeTitle: { marginTop: u(8), fontSize: u(13) },
  stats: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(4),
    marginTop: u(6),
  },
  stat: { fontSize: u(12), color: colors.white70, marginRight: u(8) },
  tags: {
    position: 'absolute',
    left: u(8),
    right: u(8),
    top: u(8),
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
  },
  tagGroup: { flexDirection: 'row', gap: u(3) },
  eventTitle: { marginTop: u(9), fontSize: u(14), lineHeight: u(18) },
  by: { marginTop: u(4), fontSize: u(13), color: colors.white70 },
});
