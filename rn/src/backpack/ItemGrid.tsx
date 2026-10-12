import { ReactElement } from 'react';
import { FlatList, StyleSheet, View } from 'react-native';
import { layout } from '../ds';

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
  bottomInset = 0,
}: {
  items: T[];
  keyOf: (item: T) => string;
  renderItem: (item: T) => ReactElement;
  header?: ReactElement;
  empty?: ReactElement;
  bottomInset?: number;
}) {
  const cells = padToColumns(items, layout.columns);
  return (
    <FlatList
      data={cells}
      numColumns={layout.columns}
      keyExtractor={(cell, index) =>
        cell === SPACER ? `spacer-${index}` : keyOf(cell as T)
      }
      ListHeaderComponent={header}
      ListEmptyComponent={empty}
      contentContainerStyle={{ paddingBottom: bottomInset + 16 }}
      columnWrapperStyle={styles.row}
      showsVerticalScrollIndicator={false}
      initialNumToRender={12}
      windowSize={7}
      renderItem={({ item }) => (
        <View style={styles.cell}>
          {item === SPACER ? null : renderItem(item as T)}
        </View>
      )}
    />
  );
}

const styles = StyleSheet.create({
  row: {
    paddingHorizontal: layout.gutter,
    gap: layout.gap,
    marginBottom: layout.gap,
  },
  cell: { flex: 1 },
});
