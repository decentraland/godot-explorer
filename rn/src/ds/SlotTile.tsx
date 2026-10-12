import { Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { GradientView } from './Gradient';
import { Icon } from './Icon';
import { colors, radii } from './tokens';

/** Category slot: small thumbnail of what is worn (or "+" when empty) with a label below. */
export function SlotTile({
  label,
  thumbnail,
  selected,
  size = 62,
  onPress,
}: {
  label: string;
  thumbnail?: string;
  selected: boolean;
  size?: number;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ selected }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.slot,
        { width: size + 6 },
        pressed && styles.pressed,
      ]}
    >
      <View
        style={[
          styles.frame,
          { width: size + 6, height: size + 6 },
          selected && styles.frameSelected,
        ]}
      >
        {thumbnail ? (
          <GradientView
            angle={225}
            stops={[
              { offset: 0, color: colors.cardLight },
              { offset: 0.65, color: colors.snow },
            ]}
            style={styles.tile}
          >
            <Image
              source={{ uri: thumbnail }}
              style={styles.image}
              resizeMode="cover"
            />
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
      <AppText variant="body" style={styles.label}>
        {label}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  slot: { alignItems: 'center' },
  frame: {
    padding: 3,
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
  label: { marginTop: 6, textAlign: 'center', fontSize: 14 },
  pressed: { opacity: 0.85 },
});
