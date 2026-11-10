import { ReactNode, useState } from 'react';
import {
  Image,
  Pressable,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { Skeleton } from './Skeleton';
import { colors, fonts, radius } from './tokens';
import { u } from './scale';

/** Card widths and image heights in dp (Figma 294×156 / 406×240). */
export const DISCOVER_CARD = {
  place: { width: 147, image: 78 },
  event: { width: 203, image: 120 },
} as const;

export type DetailType =
  | 'players'
  | 'likes'
  | 'coords'
  | 'world'
  | 'date'
  | 'duration';

const DETAIL_ICON: Record<DetailType, IconName> = {
  players: 'players',
  likes: 'like',
  coords: 'location',
  world: 'world',
  date: 'calendar',
  duration: 'clock',
};

/** Figma `Detail`: an `Icon/Base` 32 and an `Input/Value` Regular 30 value; World truncates at 148. */
export function DetailRow({
  type,
  value,
}: {
  type: DetailType;
  value: string;
}) {
  return (
    <View style={styles.detail}>
      <Icon name={DETAIL_ICON[type]} size={16} color={colors.snow} />
      <AppText
        numberOfLines={1}
        style={[styles.detailText, type === 'world' && styles.detailWorld]}
      >
        {value}
      </AppText>
    </View>
  );
}

type Common = {
  title: string;
  /** Thumbnail URL; empty or failing shows `ImageNotFound`. */
  image?: string;
  /** Drawn in place of `image` (mock art). */
  art?: ReactNode;
  onPress?: () => void;
  style?: StyleProp<ViewStyle>;
};

/** Figma `Cards-Discovery`: Places (image, title, players + likes) and Event (image with the tag
 * row, title, "by" creator). `ImageNotFound` is the deep-purple box with the crossed image. */
export function DiscoverCard(
  props: Common &
    (
      | { type: 'place'; users: string; likeRate: string }
      | { type: 'event'; byLabel: string; creator: string; tags?: ReactNode }
    ),
) {
  const { title, image, art, onPress, style } = props;
  const [failed, setFailed] = useState(false);
  const event = props.type === 'event';
  const size = DISCOVER_CARD[props.type];
  const missing = !art && (!image || failed);
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={title}
      onPress={onPress}
      style={({ pressed }) => [
        { width: u(size.width) },
        pressed && styles.pressed,
        style,
      ]}
    >
      <View
        style={[
          styles.image,
          { height: u(size.image) },
          missing &&
            (event ? styles.missingEvent : [styles.missing, styles.lavender50]),
        ]}
      >
        {art ??
          (missing ? null : (
            <Image
              source={{ uri: image }}
              style={styles.fill}
              resizeMode="cover"
              onError={() => setFailed(true)}
            />
          ))}
        {event ? (
          <View style={styles.eventInner} pointerEvents="none">
            <View style={styles.tags}>{props.tags}</View>
            {missing && (
              <View style={styles.eventIcon}>
                <Icon name="imageOff" size={40} color={colors.lavender70} />
              </View>
            )}
          </View>
        ) : (
          missing && (
            <View style={styles.centre} pointerEvents="none">
              <Icon name="imageOff" size={36} color={colors.lavender70} />
            </View>
          )
        )}
      </View>
      {props.type === 'event' ? (
        <View style={styles.eventContent}>
          <AppText numberOfLines={1} style={styles.eventTitle}>
            {title}
          </AppText>
          <View style={styles.creator}>
            <AppText style={[styles.small, styles.by]}>{props.byLabel}</AppText>
            <AppText numberOfLines={1} style={[styles.small, styles.shrink]}>
              {props.creator}
            </AppText>
          </View>
        </View>
      ) : (
        <View style={styles.placeContent}>
          <AppText numberOfLines={1} style={styles.placeTitle}>
            {title}
          </AppText>
          <View style={styles.stats}>
            <View style={styles.stat}>
              <Icon name="players" size={14} color={colors.snow} />
              <AppText style={styles.small}>{props.users}</AppText>
            </View>
            <View style={styles.stat}>
              <Icon name="like" size={14} color={colors.snow} />
              <AppText style={styles.small}>{props.likeRate}</AppText>
            </View>
          </View>
        </View>
      )}
    </Pressable>
  );
}

/** Figma `Card-Discover-Skeleton`: the loaded card's exact size, built from `Skeleton`. */
export function DiscoverCardSkeleton({ type }: { type: 'place' | 'event' }) {
  const size = DISCOVER_CARD[type];
  const event = type === 'event';
  return (
    <View style={{ width: u(size.width) }}>
      <Skeleton width="100%" height={size.image} radius={4} />
      <View style={event ? styles.eventSkeleton : styles.placeSkeleton}>
        <Skeleton width="100%" height={event ? 17 : 14} />
        <Skeleton width="100%" height={event ? 14.5 : 14} />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  pressed: { opacity: 0.85 },
  image: {
    width: '100%',
    borderRadius: radius.sm,
    borderWidth: u(0.3),
    borderColor: colors.lavender,
    overflow: 'hidden',
  },
  fill: { ...StyleSheet.absoluteFill, width: '100%', height: '100%' },
  missing: { backgroundColor: colors.deepPurple50 },
  lavender50: { borderColor: colors.lavender50 },
  missingEvent: { backgroundColor: colors.deepPurple40 },
  centre: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
  },
  eventInner: {
    flex: 1,
    paddingTop: u(8),
    paddingBottom: u(6),
    paddingHorizontal: u(8),
  },
  tags: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  eventIcon: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingBottom: u(8),
  },
  placeContent: { paddingTop: u(7), paddingBottom: u(9), gap: u(6) },
  eventContent: { paddingTop: u(10), paddingBottom: u(12), gap: u(6) },
  placeTitle: {
    fontFamily: fonts.semibold,
    fontSize: u(13),
    color: colors.snow,
  },
  eventTitle: {
    fontFamily: fonts.semibold,
    fontSize: u(14),
    color: colors.snow,
  },
  stats: { flexDirection: 'row', alignItems: 'center', gap: u(8) },
  stat: { flexDirection: 'row', alignItems: 'center', gap: u(2) },
  small: { fontFamily: fonts.regular, fontSize: u(12), color: colors.snow },
  creator: { flexDirection: 'row', gap: u(2) },
  by: { color: colors.lavender },
  shrink: { flexShrink: 1 },
  detail: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(2),
    padding: u(4),
  },
  detailText: {
    fontFamily: fonts.regular,
    fontSize: u(15),
    color: colors.snow,
  },
  detailWorld: { maxWidth: u(74) },
  placeSkeleton: { paddingTop: u(7), paddingBottom: u(9), gap: u(8) },
  eventSkeleton: { paddingTop: u(10), paddingBottom: u(12), gap: u(6) },
});
