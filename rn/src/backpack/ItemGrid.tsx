import { ReactElement } from 'react';
import { FlatList, StyleSheet, View } from 'react-native';
import { layout } from '@dcl/ui-native';

const SPACER: unique symbol = Symbol('spacer');
type Cell<T> = T | typeof SPACER;

/** Pads the last row so every cell keeps the same width. */
export function padToColumns<T>(items: T[], columns: number): Cell<T>[] {
  const missing = (columns - (items.length % columns)) % columns;
  const fillers: Cell<T>[] = Array.from({ length: missing }, () => SPACER);
  return [...items, ...fillers];
}

/** Three-column grid of square cells with an optional header (tabs, slots, ...). */
export function ItemGrid<T>({
  items,
  keyOf,
  renderItem,
  header,
  empty,
  columns = layout.columns,
  bottomInset = 0,
  overhang = 0,
  windowSize = 7,
  initialNumToRender = 12,
}: {
  items: T[];
  keyOf: (item: T) => string;
  renderItem: (item: T) => ReactElement;
  header?: ReactElement;
  empty?: ReactElement;
  columns?: number;
  bottomInset?: number;
  /** Room above the first row for card rings drawn outside the card; the list moves up by the
   * same amount so the cards stay in place. */
  overhang?: number;
  /** Screens of rows kept rendered around the visible ones (FlatList `windowSize`). */
  windowSize?: number;
  initialNumToRender?: number;
}) {
  const cells = padToColumns(items, columns);
  return (
    <FlatList
      removeClippedSubviews={false}
      data={cells}
      numColumns={columns}
      key={columns}
      keyExtractor={(cell, index) =>
        cell === SPACER ? `spacer-${index}` : keyOf(cell as T)
      }
      ListHeaderComponent={header}
      ListEmptyComponent={empty}
      style={{ marginTop: -overhang }}
      contentContainerStyle={[
        styles.content,
        { paddingTop: overhang, paddingBottom: bottomInset + 16 },
      ]}
      columnWrapperStyle={styles.row}
      showsVerticalScrollIndicator={false}
      initialNumToRender={initialNumToRender}
      windowSize={windowSize}
      renderItem={({ item }) => (
        <View style={styles.cell}>
          {item === SPACER ? null : renderItem(item as T)}
        </View>
      )}
    />
  );
}

const styles = StyleSheet.create({
  /** Lets an empty state fill the visible sheet. */
  content: { flexGrow: 1 },
  row: {
    paddingHorizontal: layout.gutter,
    gap: layout.gap,
    marginBottom: layout.gap,
  },
  cell: { flex: 1 },
});
