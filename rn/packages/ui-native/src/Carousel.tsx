import { ReactNode, useRef } from 'react';
import { Animated, StyleSheet, View } from 'react-native';
import { colors } from './tokens';
import { u } from './scale';

export function indexFromOffset(
  offset: number,
  itemWidth: number,
  gap: number,
  count: number,
): number {
  const index = Math.round(offset / (itemWidth + gap));
  return Math.min(count - 1, Math.max(0, index));
}

/** `round` keeps every dot the same size and only recolours the active one; `dark` draws the
 * inactive dots Black 40%, for the brand background. */
export function PageDots({
  count,
  index,
  round = false,
  dark = false,
}: {
  count: number;
  index: number;
  round?: boolean;
  dark?: boolean;
}) {
  return (
    <View
      style={styles.dots}
      accessibilityLabel={`Page ${index + 1} of ${count}`}
    >
      {Array.from({ length: count }, (_, i) => (
        <View
          key={i}
          accessibilityState={{ selected: i === index }}
          style={[
            styles.dot,
            dark && styles.dotDark,
            i === index && (round ? styles.dotRound : styles.dotActive),
          ]}
        />
      ))}
    </View>
  );
}

/** Horizontal carousel that settles on one item at a time. `renderItem` gets the item's `position`
 * relative to the settled one, following the scroll: 0 centred, -1 one slot to the left, 1 to the right. */
export function SnapCarousel<T>({
  data,
  itemWidth,
  gap = 12,
  keyOf,
  renderItem,
  onIndexChange,
  inset = 0,
  initialIndex = 0,
  sideScale,
}: {
  data: T[];
  itemWidth: number;
  gap?: number;
  keyOf: (item: T) => string;
  renderItem: (
    item: T,
    index: number,
    position: Animated.AnimatedInterpolation<number>,
  ) => ReactNode;
  onIndexChange?: (index: number) => void;
  /** Leading/trailing padding in dp so the first item can align with the page gutter. */
  inset?: number;
  initialIndex?: number;
  /** Scale of the items beside the settled one, eased as the carousel scrolls; they shrink
   * towards it, so the gap to it stays. */
  sideScale?: number;
}) {
  const width = u(itemWidth);
  const spacing = u(gap);
  const step = width + spacing;
  const scrollX = useRef(new Animated.Value(initialIndex * step)).current;
  const shrink = ((1 - (sideScale ?? 1)) * width) / 2;
  return (
    <Animated.ScrollView
      horizontal
      scrollEventThrottle={16}
      onScroll={Animated.event(
        [{ nativeEvent: { contentOffset: { x: scrollX } } }],
        { useNativeDriver: true },
      )}
      showsHorizontalScrollIndicator={false}
      snapToInterval={step}
      decelerationRate="fast"
      contentOffset={{ x: initialIndex * step, y: 0 }}
      contentContainerStyle={{ paddingHorizontal: u(inset), gap: spacing }}
      onMomentumScrollEnd={e =>
        onIndexChange?.(
          indexFromOffset(
            e.nativeEvent.contentOffset.x,
            width,
            spacing,
            data.length,
          ),
        )
      }
    >
      {data.map((item, i) => (
        <Animated.View
          key={keyOf(item)}
          style={[
            { width },
            sideScale !== undefined && {
              transform: [
                {
                  translateX: scrollX.interpolate({
                    inputRange: [(i - 1) * step, i * step, (i + 1) * step],
                    outputRange: [-shrink, 0, shrink],
                    extrapolate: 'clamp',
                  }),
                },
                {
                  scale: scrollX.interpolate({
                    inputRange: [(i - 1) * step, i * step, (i + 1) * step],
                    outputRange: [sideScale, 1, sideScale],
                    extrapolate: 'clamp',
                  }),
                },
              ],
            },
          ]}
        >
          {renderItem(
            item,
            i,
            scrollX.interpolate({
              inputRange: [(i - 1) * step, i * step, (i + 1) * step],
              outputRange: [1, 0, -1],
              extrapolate: 'clamp',
            }),
          )}
        </Animated.View>
      ))}
    </Animated.ScrollView>
  );
}

const styles = StyleSheet.create({
  dots: { flexDirection: 'row', justifyContent: 'center', gap: u(6) },
  dot: {
    width: u(6),
    height: u(6),
    borderRadius: u(3),
    backgroundColor: colors.white30,
  },
  dotDark: { backgroundColor: colors.black40 },
  dotRound: { backgroundColor: colors.lavender },
  dotActive: { width: u(16), backgroundColor: colors.snow },
});
