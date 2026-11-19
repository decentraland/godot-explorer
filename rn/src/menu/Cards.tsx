import { Image, StyleSheet, View } from 'react-native';
import { colors, DiscoverCard, u } from '@dcl/ui-native';
import type { DiscoverEvent, Place } from '../discover/api';
import { formatRate, formatUsers, startsIn } from '../discover/format';
import { t } from '../i18n';
import { ArtFill } from './ArtFill';
import { FeaturedTag, LiveTag, SoonTag, TrendingTag, UsersTag } from './Tags';

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
    <DiscoverCard
      type="place"
      title={place.title}
      image={place.image}
      art={place.image ? undefined : <ArtFill seed={place.title} />}
      users={formatUsers(place.users)}
      likeRate={formatRate(place.likeRate)}
      onPress={onPress}
    />
  );
}

/** The tag row shared by the event card and the event modal image. */
export function EventTags({
  event,
  featured,
  now,
  inline = false,
}: {
  event: DiscoverEvent;
  featured: boolean;
  now: number;
  /** Laid out by the parent (the card's tag row) instead of pinned over the image. */
  inline?: boolean;
}) {
  return (
    <View style={[styles.tagRow, !inline && styles.tags]} pointerEvents="none">
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
    <DiscoverCard
      type="event"
      title={event.title}
      image={event.image}
      art={event.image ? undefined : <ArtFill seed={event.title} />}
      byLabel={t('COMMON_BY')}
      creator={event.creator}
      tags={<EventTags event={event} featured={featured} now={now} inline />}
      onPress={onPress}
    />
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
  tagRow: {
    flex: 1,
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
  },
  tags: { position: 'absolute', left: u(8), right: u(8), top: u(8) },
  tagGroup: { flexDirection: 'row', gap: u(3) },
});
