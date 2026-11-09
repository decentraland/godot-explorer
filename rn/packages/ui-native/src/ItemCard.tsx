import { ReactNode } from 'react';
import {
  Image,
  Pressable,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import { AppText } from './AppText';
import { RadialFill } from './Gradient';
import { Icon } from './Icon';
import {
  colors,
  elevation,
  fonts,
  Rarity,
  rarityColors,
  radius,
} from './tokens';
import { u } from './scale';

type Glow = { color: string; cx: number; cy: number; r: number };
const fade = (color: string, opacity = 1) => [
  { offset: 0, color, opacity },
  { offset: 1, color, opacity: 0 },
];

/** Figma `Rarity-Backgrounds` (192 px box): a base colour and two radial glows, no blur. Rarities
 * glow Light top-left and base colour bottom-right (Exotic: Light twice); Default and Disable use
 * softer, 90% glows. */
function glowsFor(
  rarity: Rarity,
  disabled: boolean,
): { base: string; opacity: number; glows: [Glow, Glow] } {
  if (disabled || rarity === 'base') {
    const [base, first, second] = disabled
      ? [colors.silver, colors.gray, colors.silver]
      : [colors.cardLight, colors.snow, colors.gray];
    return {
      base,
      opacity: 0.9,
      glows: [
        { color: first, cx: 0.2903, cy: 0.3003, r: 0.6204 },
        { color: second, cx: 0.7599, cy: 0.7599, r: 0.6003 },
      ],
    };
  }
  const tone = rarityColors[rarity];
  return {
    base: colors.purple350,
    opacity: 1,
    glows: [
      { color: tone.light, cx: 0.0796, cy: 0.0406, r: 0.78 },
      {
        color: rarity === 'exotic' ? tone.light : tone.dark,
        cx: 0.7,
        cy: 0.68,
        r: 1,
      },
    ],
  };
}

export function RarityBackground({
  rarity = 'base',
  disabled = false,
  style,
}: {
  rarity?: Rarity;
  disabled?: boolean;
  style?: StyleProp<ViewStyle>;
}) {
  const { base, opacity, glows } = glowsFor(rarity, disabled);
  return (
    <View
      style={[
        StyleSheet.absoluteFill,
        { backgroundColor: base },
        disabled && styles.disabledBg,
        style,
      ]}
    >
      {glows.map((glow, i) => (
        <RadialFill
          key={i}
          stops={fade(glow.color, opacity)}
          cx={glow.cx}
          cy={glow.cy}
          rx={glow.r}
          ry={glow.r}
        />
      ))}
    </View>
  );
}

export type ItemCardState =
  | 'default'
  | 'disabled'
  | 'selected'
  | 'equipped'
  | 'equip'
  | 'unequip'
  | 'detail';

export type ItemCardProps = {
  thumbnail?: string;
  rarity?: Rarity;
  /** Figma `State`: `equip` / `unequip` / `detail` add the orange outline and the action bar
   * (`actionLabel`), `equipped` the lavender outline and the check bar, `selected` the thick orange
   * ring of the emote wheel. */
  state?: ItemCardState;
  actionLabel?: string;
  /** `Empty=On`: the empty emote-wheel slot with a plus. */
  empty?: boolean;
  /** `Premium=On`: credits price pill. */
  price?: string;
  /** `New=On`: text of the Ruby corner tag. */
  newLabel?: string;
  label?: string;
  /** Drawn in place of a missing thumbnail. */
  art?: ReactNode;
  onPress?: () => void;
  style?: StyleProp<ViewStyle>;
};

const BAR_BG = {
  equip: colors.ruby,
  unequip: colors.shadow,
  detail: colors.shadow,
} as const;

/** Figma `Cards-Wearables` (96 dp square); the grid sizes it through `style`. */
export function ItemCard({
  thumbnail,
  rarity = 'base',
  state = 'default',
  actionLabel,
  empty = false,
  price,
  newLabel,
  label,
  art,
  onPress,
  style,
}: ItemCardProps) {
  const disabled = state === 'disabled';
  const action =
    state === 'equip' || state === 'unequip' || state === 'detail'
      ? state
      : null;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{
        disabled,
        selected:
          state === 'selected' || state === 'equipped' || state === 'unequip',
      }}
      disabled={disabled || !onPress}
      onPress={onPress}
      style={[
        styles.card,
        !(empty && disabled) && elevation.down,
        empty && disabled && styles.emptyDisabled,
        style,
      ]}
    >
      {({ pressed }) => {
        const tapped = pressed && state === 'default';
        const inset = tapped || (empty && state === 'selected');
        return (
          <>
            <View
              style={[
                styles.bg,
                inset && styles.bgInset,
                empty
                  ? [
                      styles.emptyBg,
                      state === 'selected' && styles.emptySelectedBg,
                    ]
                  : styles.bgBorder,
              ]}
            >
              {empty ? (
                <Plus
                  opacity={state === 'selected' ? 0.5 : disabled ? 0.2 : 0.3}
                />
              ) : (
                <RarityBackground rarity={rarity} disabled={disabled} />
              )}
            </View>
            {!empty && (
              <View style={[styles.image, disabled && styles.dim]}>
                {thumbnail ? (
                  <Image
                    source={{ uri: thumbnail }}
                    style={styles.fill}
                    resizeMode="cover"
                  />
                ) : (
                  art ?? null
                )}
              </View>
            )}
            {tapped ? (
              <View style={[styles.ring, styles.ringPressed]} />
            ) : state === 'equipped' ? (
              <View style={[styles.ring, styles.outset, styles.ringLavender]} />
            ) : action ? (
              <View style={[styles.ring, styles.outset, styles.ringOrange]} />
            ) : state === 'selected' ? (
              <View
                style={[
                  styles.ring,
                  styles.ringSelected,
                  !empty && [styles.outset, styles.ringSelectedOutset],
                ]}
              />
            ) : null}
            {action && actionLabel ? (
              <View style={[styles.bar, { backgroundColor: BAR_BG[action] }]}>
                <AppText numberOfLines={1} style={styles.barText}>
                  {actionLabel}
                </AppText>
              </View>
            ) : state === 'equipped' ? (
              <View style={[styles.bar, styles.barEquipped]}>
                <Icon name="check" size={18} color={colors.snow} />
              </View>
            ) : null}
            {price !== undefined && !action && (
              <View style={styles.priceRow} pointerEvents="none">
                <View style={styles.price}>
                  <Icon name="credits" size={13} />
                  <AppText style={styles.priceText}>{price}</AppText>
                </View>
              </View>
            )}
            {newLabel ? (
              <View style={styles.newTag} pointerEvents="none">
                <AppText style={styles.newText}>{newLabel}</AppText>
              </View>
            ) : null}
          </>
        );
      }}
    </Pressable>
  );
}

function Plus({ opacity }: { opacity: number }) {
  return (
    <View style={[styles.plus, { opacity }]}>
      <View style={styles.plusV} />
      <View style={styles.plusH} />
    </View>
  );
}

/** Dim placeholder tile used while a list loads (Figma "skeleton"). */
export function SkeletonCard() {
  return <View style={[styles.card, styles.skeleton]} />;
}

const styles = StyleSheet.create({
  card: { aspectRatio: 1, borderRadius: radius.lg },
  bg: {
    ...StyleSheet.absoluteFill,
    borderRadius: radius.lg,
    borderWidth: 1,
    overflow: 'hidden',
  },
  bgInset: {
    top: u(2.5),
    left: u(2.5),
    right: u(2.5),
    bottom: u(2.5),
  },
  bgBorder: { borderColor: colors.snow },
  emptyBg: { borderColor: '#000000', backgroundColor: colors.black40 },
  emptySelectedBg: { backgroundColor: colors.black60 },
  emptyDisabled: { opacity: 0.6 },
  disabledBg: { opacity: 0.8 },
  image: {
    position: 'absolute',
    top: 0,
    left: u(4),
    right: u(4),
    bottom: u(7.5),
  },
  fill: { width: '100%', height: '100%' },
  dim: { opacity: 0.8 },
  ring: {
    ...StyleSheet.absoluteFill,
    borderRadius: radius.xl,
  },
  outset: {
    top: -u(2.5),
    left: -u(2.5),
    right: -u(2.5),
    bottom: -u(2.5),
  },
  ringPressed: { borderWidth: u(1.5), borderColor: colors.lavender },
  ringLavender: { borderWidth: u(1.5), borderColor: colors.lavender },
  ringOrange: { borderWidth: u(2), borderColor: colors.orange },
  ringSelected: { borderWidth: u(5), borderColor: colors.orange },
  ringSelectedOutset: { borderRadius: u(9) },
  bar: {
    position: 'absolute',
    left: 1,
    right: 1,
    bottom: 1,
    height: u(20),
    borderBottomLeftRadius: u(5),
    borderBottomRightRadius: u(5),
    alignItems: 'center',
    justifyContent: 'center',
  },
  barEquipped: { backgroundColor: colors.shadow80 },
  barText: {
    fontFamily: fonts.bold,
    fontSize: u(12),
    color: colors.snow,
    textTransform: 'uppercase',
  },
  priceRow: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: u(3.5),
    alignItems: 'center',
  },
  price: {
    height: u(18),
    paddingHorizontal: u(6),
    gap: u(3),
    borderRadius: radius.pill,
    backgroundColor: colors.black60,
    flexDirection: 'row',
    alignItems: 'center',
  },
  priceText: { fontFamily: fonts.medium, fontSize: u(12), color: colors.snow },
  newTag: {
    position: 'absolute',
    top: u(4),
    right: u(6),
    paddingHorizontal: u(2),
    borderRadius: radius.xs,
    backgroundColor: colors.ruby,
  },
  newText: {
    fontFamily: fonts.bold,
    fontSize: u(9),
    color: colors.snow,
    textTransform: 'uppercase',
  },
  plus: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    alignItems: 'center',
    justifyContent: 'center',
  },
  plusV: {
    position: 'absolute',
    width: u(2),
    height: u(16),
    backgroundColor: colors.snow,
  },
  plusH: {
    position: 'absolute',
    width: u(16),
    height: u(2),
    backgroundColor: colors.snow,
  },
  skeleton: { backgroundColor: 'rgba(105,31,169,0.28)' },
});
