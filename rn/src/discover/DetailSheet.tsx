import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Animated,
  Easing,
  Image,
  PanResponder,
  Pressable,
  Share,
  StyleSheet,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, Icon, LinearFill } from '../ds';
import { Place } from './api';
import { formatRate, formatUsers } from './format';
import { u } from '../ds/scale';

const CLOSE_DRAG = 60;
const SLIDE_MS = 220;

const jumpLink = (place: Place) =>
  `https://decentraland.org/jump/?position=${encodeURIComponent(
    place.location,
  )}`;

function SquareButton({
  icon,
  label,
  active,
  onPress,
}: {
  icon: 'share' | 'like' | 'dislike';
  label: string;
  active?: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected: !!active }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.square,
        active && styles.squareOn,
        pressed && styles.pressed,
      ]}
    >
      <Icon name={icon} size={24} strokeWidth={2} />
    </Pressable>
  );
}

/** Detail Place sheet: slides over a dimmed Discover; drag the handle down or tap outside to close. */
export function DetailSheet({
  place,
  onClose,
  onJumpIn,
}: {
  place: Place;
  onClose: () => void;
  onJumpIn: (place: Place) => void;
}) {
  const insets = useSafeAreaInsets();
  const slide = useRef(new Animated.Value(1)).current;
  const [favorite, setFavorite] = useState(false);
  const [vote, setVote] = useState<'like' | 'dislike' | null>(null);

  useEffect(() => {
    Animated.timing(slide, {
      toValue: 0,
      duration: SLIDE_MS,
      easing: Easing.out(Easing.cubic),
      useNativeDriver: true,
    }).start();
  }, [slide]);

  const close = useRef(onClose);
  close.current = onClose;
  const dismiss = useMemo(
    () => () =>
      Animated.timing(slide, {
        toValue: 1,
        duration: SLIDE_MS,
        easing: Easing.in(Easing.cubic),
        useNativeDriver: true,
      }).start(() => close.current()),
    [slide],
  );

  const responder = useMemo(
    () =>
      PanResponder.create({
        onStartShouldSetPanResponder: () => true,
        onPanResponderRelease: (_event, gesture) => {
          if (gesture.dy > CLOSE_DRAG) {
            dismiss();
          }
        },
      }),
    [dismiss],
  );

  const translateY = slide.interpolate({
    inputRange: [0, 1],
    outputRange: [0, 600],
  });
  const share = () =>
    Share.share({ message: `${place.title} ${jumpLink(place)}` }).catch(
      () => undefined,
    );

  return (
    <View style={StyleSheet.absoluteFill}>
      <Pressable
        accessibilityLabel="Close"
        style={styles.scrim}
        onPress={dismiss}
      />
      <Animated.View
        style={[
          styles.sheet,
          { paddingBottom: u(28) + insets.bottom, transform: [{ translateY }] },
        ]}
      >
        <LinearFill
          stops={[
            { offset: 0, color: colors.purple450 },
            { offset: 1, color: colors.purple550 },
          ]}
        />
        <View style={styles.handleArea} {...responder.panHandlers}>
          <View style={styles.handle} />
        </View>
        <View style={styles.head}>
          <View style={styles.headText}>
            <AppText variant="title" numberOfLines={2} style={styles.title}>
              {place.title}
            </AppText>
            {place.creator ? (
              <AppText variant="body" style={styles.by}>
                by{' '}
                <AppText variant="body" style={styles.creator}>
                  {place.creator}
                </AppText>
              </AppText>
            ) : null}
          </View>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Favorite"
            accessibilityState={{ selected: favorite }}
            hitSlop={10}
            onPress={() => setFavorite(on => !on)}
          >
            <Icon
              name="heart"
              size={30}
              color={favorite ? colors.ruby : colors.snow}
              strokeWidth={2}
            />
          </Pressable>
        </View>
        <View style={styles.image}>
          {place.image ? (
            <Image
              source={{ uri: place.image }}
              style={styles.fill}
              resizeMode="cover"
            />
          ) : null}
        </View>
        <View style={styles.stats}>
          <View style={styles.stat}>
            <Icon name="user" size={15} color={colors.gray} strokeWidth={2} />
            <AppText variant="body" style={styles.statText}>
              {formatUsers(place.users)}
            </AppText>
          </View>
          <View style={styles.divider} />
          <View style={styles.stat}>
            <Icon name="like" size={15} color={colors.gray} strokeWidth={2} />
            <AppText variant="body" style={styles.statText}>
              {formatRate(place.likeRate)}
            </AppText>
          </View>
          <View style={styles.divider} />
          <View style={styles.stat}>
            <Icon name="pin" size={15} color={colors.gray} strokeWidth={2} />
            <AppText variant="body" style={styles.statText}>
              {place.location || '—'}
            </AppText>
          </View>
        </View>
        <View style={styles.actions}>
          <SquareButton icon="share" label="Share" onPress={share} />
          <SquareButton
            icon="like"
            label="Like"
            active={vote === 'like'}
            onPress={() => setVote(v => (v === 'like' ? null : 'like'))}
          />
          <SquareButton
            icon="dislike"
            label="Dislike"
            active={vote === 'dislike'}
            onPress={() => setVote(v => (v === 'dislike' ? null : 'dislike'))}
          />
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Jump in"
            onPress={() => onJumpIn(place)}
            style={({ pressed }) => [styles.jump, pressed && styles.pressed]}
          >
            <AppText variant="button" style={styles.jumpText}>
              JUMP IN
            </AppText>
            <Icon name="jump" size={16} strokeWidth={2.4} />
          </Pressable>
        </View>
      </Animated.View>
    </View>
  );
}

const styles = StyleSheet.create({
  scrim: { ...StyleSheet.absoluteFill, backgroundColor: 'rgba(24,3,48,0.72)' },
  sheet: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    borderTopLeftRadius: u(24),
    borderTopRightRadius: u(24),
    overflow: 'hidden',
    backgroundColor: colors.purple550,
  },
  handleArea: { height: u(28), alignItems: 'center', justifyContent: 'center' },
  handle: {
    width: u(100),
    height: u(4),
    borderRadius: u(2),
    backgroundColor: 'rgba(252,252,252,0.3)',
  },
  head: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    paddingHorizontal: u(24),
    marginTop: u(6),
  },
  headText: { flex: 1, paddingRight: u(16) },
  title: { fontSize: u(24), lineHeight: u(30), fontFamily: 'Inter-SemiBold' },
  by: { marginTop: u(8), fontSize: u(14), color: colors.gray },
  creator: {
    fontSize: u(14),
    color: colors.lavender,
    fontFamily: 'Inter-Medium',
  },
  image: {
    height: u(184),
    marginHorizontal: u(24),
    marginTop: u(18),
    borderRadius: u(6),
    overflow: 'hidden',
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.35)',
    backgroundColor: colors.purple450,
  },
  fill: { width: '100%', height: '100%' },
  stats: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginHorizontal: u(24),
    marginTop: u(18),
  },
  stat: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(6),
  },
  statText: { fontSize: u(15), color: colors.pearl },
  divider: { width: u(1), height: u(24), backgroundColor: colors.white30 },
  actions: {
    flexDirection: 'row',
    gap: u(8),
    marginHorizontal: u(24),
    marginTop: u(22),
  },
  square: {
    width: u(47),
    height: u(47),
    borderRadius: u(6),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.purple450,
  },
  squareOn: { borderWidth: 2, borderColor: colors.ruby },
  jump: {
    flex: 1,
    height: u(47),
    borderRadius: u(6),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(8),
    backgroundColor: colors.ruby,
  },
  jumpText: { fontFamily: 'Inter-Bold', fontSize: u(16) },
  pressed: { opacity: 0.85 },
});
