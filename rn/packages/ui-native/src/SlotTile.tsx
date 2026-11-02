import { ReactNode } from 'react';
import { Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { GradientView } from './Gradient';
import { Icon } from './Icon';
import { colors, radii } from './tokens';
import { u } from './scale';

/** Category slot: small thumbnail of what is worn (or "+" when empty) with a label below. */
export function SlotTile({
  label,
  thumbnail,
  selected,
  size: designSize = 62,
  art,
  onPress,
}: {
  label: string;
  thumbnail?: string;
  selected: boolean;
  size?: number;
  /** Drawn in the tile when there is no thumbnail but the slot is worn. */
  art?: ReactNode;
  onPress: () => void;
}) {
  const size = u(designSize);
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ selected }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.slot,
        { width: size + u(6) },
        pressed && styles.pressed,
      ]}
    >
      <View
        style={[
          styles.frame,
          { width: size + u(6), height: size + u(6) },
          selected && styles.frameSelected,
        ]}
      >
        {thumbnail || art ? (
          <GradientView
            angle={225}
            stops={[
              { offset: 0, color: colors.cardLight },
              { offset: 0.65, color: colors.snow },
            ]}
            style={styles.tile}
          >
            {thumbnail ? (
              <Image
                source={{ uri: thumbnail }}
                style={styles.image}
                resizeMode="cover"
              />
            ) : (
              art
            )}
          </GradientView>
        ) : (
          <View style={[styles.tile, styles.empty]}>
            <Icon
              name="plus"
              size={18}
              color={colors.silver}
              strokeWidth={1.6}
            />
          </View>
        )}
      </View>
      <AppText variant="body" numberOfLines={1} style={styles.label}>
        {label}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  slot: { alignItems: 'center' },
  frame: {
    padding: u(3),
    borderRadius: radii.card + 2,
    borderWidth: 2,
    borderColor: 'transparent',
  },
  frameSelected: { borderColor: colors.orange },
  tile: { flex: 1, borderRadius: radii.card, overflow: 'hidden' },
  image: { ...StyleSheet.absoluteFill, width: '100%', height: '100%' },
  empty: {
    backgroundColor: colors.purple550,
    alignItems: 'center',
    justifyContent: 'center',
  },
  label: {
    marginTop: u(6),
    textAlign: 'center',
    fontSize: u(14),
    width: u(88),
  },
  pressed: { opacity: 0.85 },
});
