import { ReactNode } from 'react';
import { ScrollView, StyleSheet, View } from 'react-native';
import { AppText, colors } from '../ds';
import { u } from '../ds/scale';
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
    <AppText variant="title" style={styles.title}>
      {title}
      {count !== undefined ? (
        <AppText variant="title" style={styles.count}>
          {` ${count}`}
        </AppText>
      ) : null}
    </AppText>
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
}: {
  title: string;
  count?: boolean;
  state: RailState<T>;
  keyOf: (item: T, index: number) => string;
  renderItem: (item: T) => ReactNode;
  skeleton?: 'place' | 'event' | 'compact';
  loadingMore?: boolean;
  onEnd?: () => void;
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
    <View style={[styles.section, skeleton !== 'compact' && styles.cards]}>
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
        contentContainerStyle={styles.rail}
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
  section: { marginTop: u(18) },
  cards: { paddingBottom: u(8) },
  title: { fontSize: u(16), paddingHorizontal: u(24), marginBottom: u(8.5) },
  count: { fontSize: u(16), color: colors.lavender },
  rail: { paddingHorizontal: u(24), gap: u(16) },
});
