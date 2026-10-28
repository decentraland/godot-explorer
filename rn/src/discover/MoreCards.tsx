import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Skeleton } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { Place } from './api';
import { EVENT_W, PLACE_W } from './Cards';
import { Thumb } from './Thumb';
import type { Friend } from './types';

const AVATAR = 80;

/** "Jump back in" compact row: square thumbnail and a two-line title. */
export function CompactRow({
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
      style={({ pressed }) => [styles.compact, pressed && styles.pressed]}
    >
      <Thumb uri={place.image} width={u(60)} height={u(60)} radius={4} />
      <View style={styles.compactText}>
        <AppText variant="button" numberOfLines={2} style={styles.compactTitle}>
          {place.title}
        </AppText>
      </View>
    </Pressable>
  );
}

/** Round avatar with a white ring; the engine thumbnail replaces the initial in wave 3. */
export function FriendCard({
  friend,
  onPress,
}: {
  friend: Friend;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={friend.name}
      onPress={onPress}
      style={({ pressed }) => [styles.friend, pressed && styles.pressed]}
    >
      <View style={[styles.avatar, { backgroundColor: friend.color }]}>
        <AppText variant="heading" style={styles.initial}>
          {friend.name.charAt(0).toUpperCase()}
        </AppText>
      </View>
      <AppText variant="button" numberOfLines={1} style={styles.friendName}>
        {friend.name}
      </AppText>
      {friend.place ? (
        <AppText variant="caption" numberOfLines={2} style={styles.friendPlace}>
          {friend.place.title}
        </AppText>
      ) : null}
    </Pressable>
  );
}

/** Trailing skeleton card shown at the end of a rail while the next page loads. */
export function LoadMoreCard({ wide = false }: { wide?: boolean }) {
  return (
    <View accessibilityLabel={t('DISCOVER_LOAD_MORE')}>
      <PlaceSkeleton wide={wide} />
    </View>
  );
}

export function PlaceSkeleton({ wide = false }: { wide?: boolean }) {
  const width = wide ? EVENT_W : PLACE_W;
  return (
    <View style={{ width: u(width) }}>
      <Skeleton width={u(width)} height={wide ? 120 : 78} radius={4} />
      <Skeleton
        width={u(width)}
        height={wide ? 17 : 14}
        radius={3}
        style={styles.bar}
      />
      <Skeleton width={u(width)} height={14} radius={3} style={styles.bar} />
    </View>
  );
}

const styles = StyleSheet.create({
  compact: {
    width: u(206),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(10),
  },
  compactText: { flex: 1 },
  compactTitle: { fontSize: u(12), lineHeight: u(16) },
  friend: { width: u(124), alignItems: 'center' },
  avatar: {
    width: u(AVATAR),
    height: u(AVATAR),
    borderRadius: u(AVATAR / 2),
    borderWidth: 2,
    borderColor: colors.snow,
    alignItems: 'center',
    justifyContent: 'center',
  },
  initial: { color: colors.purple450, fontSize: u(28) },
  friendName: { marginTop: u(6), fontSize: u(14), fontFamily: 'Inter-Bold' },
  friendPlace: {
    marginTop: u(2),
    fontSize: u(12),
    lineHeight: u(13.5),
    textAlign: 'center',
    color: colors.white70,
  },
  bar: { marginTop: u(7) },
  pressed: { opacity: 0.85 },
});
