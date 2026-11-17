import { ReactNode, useState } from 'react';
import { Animated, Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { LinearFill } from './Gradient';
import { Icon } from './Icon';
import { t } from './i18n';
import { alpha, color, colors, radius, space } from './tokens';
import { u } from './scale';

/** Selected 624×350 px in design dp (like `DISCOVER_CARD`); Default (the cards beside it) is 530×300. */
export const FEATURE = {
  width: 312,
  height: 175,
  defaultScaleX: 530 / 624,
  defaultScaleY: 300 / 350,
};

const GRADIENT = [
  { offset: 0, color: colors.shadow, opacity: 0 },
  { offset: 1, color: alpha['shadow-90'] },
];

/** A featured place: the image with, when Selected, the name and the Jump in button over a bottom
 * gradient. Tapping the card opens it (`onPress`); the button jumps straight in (`onJumpIn`).
 * `transition` (0 Default → 1 Selected) lets a carousel move between the two states.
 * Figma Feature: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=819-3463
 */
export function Feature({
  state = 'Selected',
  title,
  image,
  art,
  onPress,
  onJumpIn,
  transition,
}: {
  state?: 'Selected' | 'Default';
  title: string;
  image?: string;
  /** Drawn instead of `image` (previews). */
  art?: ReactNode;
  onPress: () => void;
  onJumpIn: () => void;
  transition?: Animated.AnimatedInterpolation<number>;
}) {
  const [failed, setFailed] = useState(false);
  const selected = state === 'Selected';
  const missing = !art && (!image || failed);
  const amount = transition ?? (selected ? 1 : 0);
  const scale = (to: number) =>
    typeof amount === 'number'
      ? to + (1 - to) * amount
      : amount.interpolate({ inputRange: [0, 1], outputRange: [to, 1] });
  return (
    <Animated.View
      style={{
        transform: [
          { scaleX: scale(FEATURE.defaultScaleX) },
          { scaleY: scale(FEATURE.defaultScaleY) },
        ],
      }}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={title}
        onPress={onPress}
        style={[styles.card, missing && styles.missing]}
      >
        {art ??
          (missing ? (
            <View style={styles.centre}>
              <Icon name="imageOff" size={36} color={colors.lavender70} />
            </View>
          ) : (
            <Image
              source={{ uri: image }}
              style={StyleSheet.absoluteFill}
              resizeMode="cover"
              onError={() => setFailed(true)}
            />
          ))}
        <Animated.View
          style={[StyleSheet.absoluteFill, { opacity: amount }]}
          pointerEvents={selected ? 'box-none' : 'none'}
        >
          <View style={styles.gradient} pointerEvents="none">
            <LinearFill stops={GRADIENT} angle={0} />
          </View>
          <View style={styles.row} pointerEvents="box-none">
            <AppText variant="Pill/Large" numberOfLines={2} style={styles.name}>
              {title}
            </AppText>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={t('APP_JUMP_IN')}
              hitSlop={u(8)}
              onPress={onJumpIn}
              style={({ pressed }) => [
                styles.jump,
                pressed && { backgroundColor: color.action['primary-pressed'] },
              ]}
            >
              <Icon name="jumpIn" size={12} />
            </Pressable>
          </View>
        </Animated.View>
        <View style={styles.border} pointerEvents="none" />
      </Pressable>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  card: {
    width: u(FEATURE.width),
    height: u(FEATURE.height),
    borderRadius: radius.sm,
    overflow: 'hidden',
  },
  missing: { backgroundColor: colors.deepPurple50 },
  centre: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
  },
  border: {
    ...StyleSheet.absoluteFill,
    borderRadius: radius.sm,
    borderWidth: u(0.5),
    borderColor: colors.lavender,
  },
  gradient: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: u(84.5),
  },
  row: {
    position: 'absolute',
    top: u(113.5),
    left: 0,
    right: 0,
    flexDirection: 'row',
    alignItems: 'center',
    gap: space.xl,
    paddingHorizontal: space.xl,
    paddingVertical: space['30'],
  },
  name: { flex: 1, color: color.text.primary },
  jump: {
    width: u(27),
    height: u(27),
    borderRadius: radius.sm,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: color.action.primary,
  },
});
