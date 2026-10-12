import { Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { GradientView } from './Gradient';
import { colors, Rarity, rarityColors, radii } from './tokens';

export type ItemCardProps = {
  thumbnail?: string;
  rarity?: Rarity;
  equipped?: boolean;
  /** Caption strip along the bottom edge (Figma: UNEQUIP on equipped items). */
  caption?: string;
  label?: string;
  onPress?: () => void;
};

/** Square item tile; the grid sizes it with `style` (width via flexBasis, aspectRatio 1). */
export function ItemCard({
  thumbnail,
  rarity = 'base',
  equipped = false,
  caption,
  label,
  onPress,
}: ItemCardProps) {
  const tone = rarityColors[rarity];
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected: equipped }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.outer,
        equipped && styles.equipped,
        pressed && styles.pressed,
      ]}
    >
      <GradientView
        angle={225}
        stops={[
          { offset: 0, color: tone.dark },
          { offset: 0.65, color: tone.light },
        ]}
        style={styles.fill}
      >
        {thumbnail ? (
          <Image
            source={{ uri: thumbnail }}
            style={styles.image}
            resizeMode="cover"
          />
        ) : null}
        {equipped && caption ? (
          <View style={styles.caption}>
            <AppText variant="button" style={styles.captionText}>
              {caption}
            </AppText>
          </View>
        ) : null}
      </GradientView>
    </Pressable>
  );
}

/** Dim placeholder tile used while a list loads (Figma "skeleton"). */
export function SkeletonCard() {
  return <View style={[styles.outer, styles.skeleton]} />;
}

const styles = StyleSheet.create({
  outer: {
    aspectRatio: 1,
    borderRadius: radii.card,
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.8)',
    overflow: 'hidden',
  },
  equipped: { borderWidth: 2, borderColor: colors.orange },
  fill: { flex: 1 },
  image: { ...StyleSheet.absoluteFill, width: '100%', height: '100%' },
  caption: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: 22,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.shadow,
  },
  captionText: {
    fontFamily: 'Inter-Bold',
    fontSize: 14,
    textTransform: 'uppercase',
  },
  skeleton: {
    borderColor: 'transparent',
    backgroundColor: 'rgba(105,31,169,0.28)',
  },
  pressed: { opacity: 0.85 },
});
