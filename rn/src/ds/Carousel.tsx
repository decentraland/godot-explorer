import { ReactNode } from 'react';
import { ScrollView, StyleSheet, View } from 'react-native';
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

/** `round` keeps every dot the same size and only recolours the active one. */
export function PageDots({
  count,
  index,
  round = false,
}: {
  count: number;
  index: number;
  round?: boolean;
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
            i === index && (round ? styles.dotRound : styles.dotActive),
          ]}
        />
      ))}
    </View>
  );
}

/** Horizontal carousel that settles on one item at a time. */
export function SnapCarousel<T>({
  data,
  itemWidth,
  gap = 12,
  keyOf,
  renderItem,
  onIndexChange,
  inset = 0,
}: {
  data: T[];
  itemWidth: number;
  gap?: number;
  keyOf: (item: T) => string;
  renderItem: (item: T, index: number) => ReactNode;
  onIndexChange?: (index: number) => void;
  /** Leading/trailing padding in dp so the first item can align with the page gutter. */
  inset?: number;
}) {
  const width = u(itemWidth);
  const spacing = u(gap);
  return (
    <ScrollView
      horizontal
      showsHorizontalScrollIndicator={false}
      snapToInterval={width + spacing}
      decelerationRate="fast"
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
        <View key={keyOf(item)} style={{ width }}>
          {renderItem(item, i)}
        </View>
      ))}
    </ScrollView>
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
  dotRound: { backgroundColor: colors.lavender },
  dotActive: { width: u(16), backgroundColor: colors.snow },
});
