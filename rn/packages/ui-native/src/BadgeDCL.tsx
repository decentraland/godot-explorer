import {
  Image,
  ImageSourcePropType,
  Pressable,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import { AppText } from './AppText';
import { LinearFill } from './Gradient';
import { ItemCard } from './ItemCard';
import { alpha, color, colors, radius, space } from './tokens';
import { u } from './scale';

export type BadgeState = 'default' | 'selected' | 'disabled';

const PROGRESS = [
  { offset: 0, color: colors.ruby },
  { offset: 1, color: '#FFBC5B' },
];
const COMPLETED = [
  { offset: 0, color: '#34CE76' },
  { offset: 1, color: '#73FFAF' },
];

/** Figma `Bar`: tier progress under a badge; `value` is 0..1, green when complete.
 * Figma Bar: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1182-4565
 */
export function BadgeBar({
  value,
  style,
}: {
  value: number;
  style?: StyleProp<ViewStyle>;
}) {
  const clamped = Math.min(1, Math.max(0, value));
  return (
    <View
      accessibilityRole="progressbar"
      accessibilityValue={{ min: 0, max: 100, now: Math.round(clamped * 100) }}
      style={[styles.track, style]}
    >
      {clamped > 0 && (
        <View style={[styles.fill, { width: `${clamped * 100}%` }]}>
          <LinearFill
            angle={90}
            radius={radius.pill}
            stops={clamped >= 1 ? COMPLETED : PROGRESS}
          />
        </View>
      )}
    </View>
  );
}

/** Figma `Badge-DCL`: an achievement card (90 × 110 dp) with its art, name and either the tier
 * progress or the date it was earned.
 * Figma Badge-DCL: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1182-4577
 */
export function BadgeDCL({
  image,
  name,
  state = 'default',
  progress,
  date,
  onPress,
  style,
}: {
  image: ImageSourcePropType;
  name: string;
  state?: BadgeState;
  /** 0..1; shows the `Bar`. */
  progress?: number;
  /** Shown under the name when there is no `progress`. */
  date?: string;
  onPress?: () => void;
  style?: StyleProp<ViewStyle>;
}) {
  const disabled = state === 'disabled';
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ selected: state === 'selected', disabled }}
      accessibilityLabel={name}
      disabled={!onPress}
      onPress={onPress}
      style={[
        styles.card,
        state === 'selected' ? styles.cardSelected : styles.cardDefault,
        style,
      ]}
    >
      <Image
        source={image}
        style={[styles.art, disabled && styles.artDisabled]}
      />
      <View style={styles.name}>
        <AppText
          variant="Tags"
          numberOfLines={2}
          style={[styles.title, disabled && styles.titleDisabled]}
        >
          {name}
        </AppText>
        {progress !== undefined ? (
          <BadgeBar value={progress} />
        ) : (
          !!date && (
            <AppText variant="Badge/Date" style={styles.date}>
              {date}
            </AppText>
          )
        )}
      </View>
      {state === 'selected' && (
        <View pointerEvents="none" style={styles.outline} />
      )}
    </Pressable>
  );
}

/** Figma `Badge-Small` (25 dp): a badge thumbnail; `size` scales it (the profile row uses 50).
 * Figma Badge-Small: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1182-4611
 */
export function BadgeSmall({
  image,
  state = 'default',
  size = 25,
  onPress,
}: {
  image: ImageSourcePropType;
  state?: BadgeState;
  size?: number;
  onPress?: () => void;
}) {
  const k = size / 25;
  const ring = u(1.25) * k;
  const gap = u(1.5) * k;
  const inset = (state === 'disabled' ? u(4.5) : u(2)) * k;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{
        selected: state === 'selected',
        disabled: state === 'disabled',
      }}
      disabled={!onPress}
      onPress={onPress}
      style={[
        styles.small,
        { width: u(size), height: u(size), borderRadius: radius.xs * k },
      ]}
    >
      <Image
        source={image}
        style={[
          styles.smallArt,
          {
            top: inset,
            left: inset,
            width: u(size) - 2 * inset,
            height: u(size) - 2 * inset,
          },
          state === 'disabled' && styles.smallArtDisabled,
        ]}
      />
      {state === 'selected' && (
        <View
          pointerEvents="none"
          style={[
            styles.smallOutline,
            {
              top: -(gap + ring),
              left: -(gap + ring),
              right: -(gap + ring),
              bottom: -(gap + ring),
              borderWidth: ring,
              borderRadius: radius.xs * k + ring,
            },
          ]}
        />
      )}
    </Pressable>
  );
}

/** Figma `Badge-ProfileRow`: a 50 dp slot of the profile's badge row; empty without `image`.
 * Figma Badge-ProfileRow: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1182-4621
 */
export function BadgeProfileRow({
  image,
  selected = false,
  onPress,
}: {
  image?: ImageSourcePropType;
  selected?: boolean;
  onPress?: () => void;
}) {
  if (!image) {
    return <ItemCard empty onPress={onPress} style={styles.emptySlot} />;
  }
  return (
    <BadgeSmall
      image={image}
      size={50}
      state={selected ? 'selected' : 'default'}
      onPress={onPress}
    />
  );
}

const styles = StyleSheet.create({
  track: {
    width: u(50),
    height: u(4),
    borderRadius: radius.pill,
    backgroundColor: alpha['white-10'],
  },
  fill: {
    height: '100%',
    borderRadius: radius.pill,
    boxShadow: [
      {
        offsetX: u(0.5),
        offsetY: 0,
        blurRadius: u(0.5),
        color: colors.black30,
      },
    ],
  },
  card: {
    width: u(90),
    height: u(110),
    borderRadius: radius.lg,
    paddingVertical: u(11),
    gap: space['14'],
    alignItems: 'center',
  },
  cardDefault: { backgroundColor: color.surface.overlay },
  cardSelected: {
    backgroundColor: color.surface['inverse-disabled'],
    boxShadow: [
      {
        inset: true,
        offsetX: 0,
        offsetY: 0,
        blurRadius: u(1),
        color: colors.black50,
      },
    ],
  },
  art: { width: u(43), height: u(43) },
  artDisabled: { opacity: 0.3 },
  name: { alignSelf: 'stretch', alignItems: 'center', gap: space.md },
  title: { width: u(63), textAlign: 'center', color: color.text.primary },
  titleDisabled: { color: color.text.disabled },
  date: { alignSelf: 'stretch', textAlign: 'center', color: colors.white70 },
  outline: {
    position: 'absolute',
    top: -u(2.5),
    left: -u(2.5),
    right: -u(2.5),
    bottom: -u(2.5),
    borderWidth: u(2),
    borderRadius: radius.xl,
    borderColor: colors.orange,
  },
  small: { backgroundColor: color.surface.overlay },
  smallArt: { position: 'absolute' },
  smallArtDisabled: { opacity: 0.2 },
  smallOutline: { position: 'absolute', borderColor: colors.orange },
  emptySlot: { width: u(50), height: u(50) },
});
