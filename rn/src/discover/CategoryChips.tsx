import { Image, Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText, categoryTile, fonts, RadialFill, u } from '@dcl/ui-native';
import type { Category } from './types';

// A disc of radius 41 blurred by 14 inside its 138px box, sampled along the radius.
/** Figma blends the shadow with multiply; over the tile's opaque fill that is the per-channel product,
 * drawn normally so every platform gets it (`mixBlendMode` did not apply on device). */
function multiply(base: string, over: string): string {
  const channel = (hex: string, i: number) =>
    parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16);
  const out = [0, 1, 2].map(i =>
    Math.round((channel(base, i) * channel(over, i)) / 255)
      .toString(16)
      .padStart(2, '0'),
  );
  return `#${out.join('')}`;
}

const SHADOW_FALLOFF = [
  [0, 0.99],
  [0.3, 0.89],
  [0.45, 0.69],
  [0.6, 0.42],
  [0.7, 0.25],
  [0.8, 0.12],
  [0.9, 0.05],
  [1, 0],
];

function CategoryTile({
  category,
  onPress,
}: {
  category: Category;
  onPress: () => void;
}) {
  const tone = categoryTile[category.tile];
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={category.name}
      onPress={onPress}
      style={({ pressed }) => [styles.chip, pressed && styles.pressed]}
    >
      <View
        style={[
          styles.tile,
          {
            backgroundColor: tone.fill,
            boxShadow: [
              {
                inset: true,
                offsetX: 0,
                offsetY: 0,
                blurRadius: u(4),
                spreadDistance: u(1),
                color: tone.glow,
              },
            ],
          },
        ]}
      >
        <View style={[styles.shadow, { opacity: tone.shadowOpacity }]}>
          <RadialFill
            stops={SHADOW_FALLOFF.map(([offset, opacity]) => ({
              offset,
              opacity,
              color: multiply(tone.fill, tone.shadow),
            }))}
          />
        </View>
        <Image
          source={category.art}
          resizeMode="cover"
          style={category.tile === 'social' ? styles.artLarge : styles.art}
        />
      </View>
      <AppText variant="label" style={styles.label}>
        {category.name}
      </AppText>
    </Pressable>
  );
}

/** Figma `Categories Carousel`: four tiles spread across the width, or, with the seasonal tile
 * first, a row that scrolls. */
export function CategoryChips({
  categories,
  onOpen,
}: {
  categories: Category[];
  onOpen: (category: Category) => void;
}) {
  const tiles = categories.map(category => (
    <CategoryTile
      key={category.id}
      category={category}
      onPress={() => onOpen(category)}
    />
  ));
  if (categories.length <= 4) {
    return <View style={[styles.row, styles.spread]}>{tiles}</View>;
  }
  return (
    <ScrollView
      horizontal
      showsHorizontalScrollIndicator={false}
      style={styles.scroll}
      contentContainerStyle={[styles.row, styles.gap]}
    >
      {tiles}
    </ScrollView>
  );
}

const TILE = u(60);
const SHADOW = u(69);

const styles = StyleSheet.create({
  scroll: { flexGrow: 0 },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: u(20),
    paddingTop: u(10),
    paddingBottom: u(4),
    paddingHorizontal: u(24),
  },
  spread: { justifyContent: 'space-between' },
  gap: { gap: u(19) },
  chip: { alignItems: 'center', gap: u(4) },
  tile: {
    width: TILE,
    height: TILE,
    borderRadius: u(7),
    overflow: 'hidden',
    alignItems: 'center',
    justifyContent: 'center',
  },
  shadow: {
    position: 'absolute',
    width: SHADOW,
    height: SHADOW,
    left: (TILE - SHADOW) / 2,
    top: (TILE - SHADOW) / 2 + u(0.5),
  },
  art: { width: u(42), height: u(42) },
  artLarge: { width: u(44), height: u(44) },
  label: { fontFamily: fonts.semibold, fontSize: u(9), lineHeight: u(11) },
  pressed: { opacity: 0.85 },
});
