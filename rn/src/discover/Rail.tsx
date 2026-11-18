import { ReactNode } from 'react';
import { ScrollView, StyleSheet, View } from 'react-native';
import { AppText, color, colors, space, u } from '@dcl/ui-native';
import { LoadMoreCard, PlaceSkeleton } from './MoreCards';
import type { RailState } from './rails';

export function SectionTitle({
  title,
  count,
}: {
  title: string;
  count?: number;
}) {
  return (
    <View style={styles.titleRow}>
      <AppText variant="H3" style={styles.title}>
        {title}
        {count !== undefined ? (
          <AppText variant="H3" style={styles.count}>
            {` ${count}`}
          </AppText>
        ) : null}
      </AppText>
    </View>
  );
}

/**
 * Titled horizontal rail that loads on its own: skeletons while loading, hidden when it has nothing
 * (or failed), a trailing skeleton card while the next page loads.
 */
export function Rail<T>({
  title,
  count,
  state,
  keyOf,
  renderItem,
  skeleton = 'place',
  loadingMore = false,
  onEnd,
  gap = 16,
}: {
  title: string;
  count?: boolean;
  state: RailState<T>;
  keyOf: (item: T, index: number) => string;
  renderItem: (item: T) => ReactNode;
  skeleton?: 'place' | 'event' | 'compact';
  loadingMore?: boolean;
  onEnd?: () => void;
  /** Space between cards, in Figma dp. */
  gap?: number;
}) {
  if (
    state.status === 'error' ||
    (state.status === 'ready' && state.items.length === 0)
  ) {
    return null;
  }
  const loading = state.status === 'loading';
  const wide = skeleton === 'event';
  return (
    <View style={styles.section}>
      <SectionTitle
        title={title}
        count={
          count && state.status === 'ready' ? state.items.length : undefined
        }
      />
      <ScrollView
        horizontal
        removeClippedSubviews={false}
        showsHorizontalScrollIndicator={false}
        contentContainerStyle={[styles.rail, { gap: u(gap) }]}
        scrollEventThrottle={64}
        onScroll={e => {
          const { contentOffset, contentSize, layoutMeasurement } =
            e.nativeEvent;
          if (
            contentOffset.x + layoutMeasurement.width >
            contentSize.width - u(60)
          ) {
            onEnd?.();
          }
        }}
      >
        {loading ? (
          <>
            <PlaceSkeleton wide={wide} />
            <PlaceSkeleton wide={wide} />
          </>
        ) : (
          <>
            {state.items.map((item, i) => (
              <View key={keyOf(item, i)}>{renderItem(item)}</View>
            ))}
            {loadingMore || state.hasMore ? <LoadMoreCard wide={wide} /> : null}
          </>
        )}
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  // Figma: 40px between sections; the title sits in a 40px row, 20px above the cards.
  section: { marginTop: space['40'] },
  titleRow: {
    height: space['40'],
    justifyContent: 'center',
    paddingHorizontal: space['2xl'],
    marginBottom: space['20'],
  },
  title: { color: color.text.secondary },
  count: { color: colors.lavender },
  rail: { paddingHorizontal: u(24) },
});
