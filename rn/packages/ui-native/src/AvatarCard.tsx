import { ReactNode, useId } from 'react';
import {
  Image,
  Pressable,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import Svg, { Defs, LinearGradient, Rect, Stop } from 'react-native-svg';
import { colors, elevation, lightRainbow, radius } from './tokens';
import { u } from './scale';

const W = u(80);
const H = u(180);
const RING = u(2);

/** Figma `Avatar-Card` (160×360): Default Black 40%, Pressed Black 50% + a 1 px Lavender Light
 * border, Selected Black 70% + a 4 px Light Rainbow border; the avatar fills it with 10% margins. */
export function AvatarCard({
  image,
  art,
  selected = false,
  label,
  onPress,
  style,
}: {
  image?: string;
  /** Drawn in place of `image` (e.g. a live avatar preview). */
  art?: ReactNode;
  selected?: boolean;
  label?: string;
  onPress?: () => void;
  style?: StyleProp<ViewStyle>;
}) {
  const id = useId();
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected }}
      onPress={onPress}
      style={[styles.card, elevation.down, style]}
    >
      {({ pressed }) => (
        <>
          <View
            style={[
              styles.back,
              selected
                ? styles.backSelected
                : pressed
                ? styles.backPressed
                : null,
            ]}
          />
          {selected && (
            <Svg width={W} height={H} style={StyleSheet.absoluteFill}>
              <Defs>
                {/* CSS 147.6°: from the top-left corner region to the bottom-right. */}
                <LinearGradient id={id} x1="0.23" y1="0" x2="0.77" y2="1">
                  {lightRainbow.map(stop => (
                    <Stop
                      key={stop.offset}
                      offset={stop.offset}
                      stopColor={stop.color}
                    />
                  ))}
                </LinearGradient>
              </Defs>
              <Rect
                x={RING / 2}
                y={RING / 2}
                width={W - RING}
                height={H - RING}
                rx={radius.xl - RING / 2}
                fill="none"
                stroke={`url(#${id})`}
                strokeWidth={RING}
              />
            </Svg>
          )}
          <View style={styles.image}>
            {image ? (
              <Image
                source={{ uri: image }}
                style={styles.fill}
                resizeMode="contain"
              />
            ) : (
              art ?? null
            )}
          </View>
        </>
      )}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  card: { width: W, height: H, borderRadius: radius.xl },
  back: {
    ...StyleSheet.absoluteFill,
    borderRadius: radius.xl,
    backgroundColor: colors.black40,
  },
  backPressed: {
    backgroundColor: colors.black50,
    borderWidth: u(0.5),
    borderColor: colors.lavender,
  },
  backSelected: { backgroundColor: colors.black70 },
  image: {
    position: 'absolute',
    left: 0,
    right: 0,
    top: '10.14%',
    bottom: '10.14%',
  },
  fill: { width: '100%', height: '100%' },
});
