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
import { AngularPaths, AngularStop } from './internal/AngularStroke';
import { CachedSvg } from './internal/CachedSvg';
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

/**
 * Figma Rarity-Backgrounds: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=785-970
 */
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

/** `#RRGGBB` at an opacity, as `#RRGGBBAA`. */
const withAlpha = (hex: string, alpha: number) =>
  `${hex}${Math.round(alpha * 255)
    .toString(16)
    .padStart(2, '0')}`;

const faceStyles = new Map<string, ViewStyle>();

/** The rarity glows as native gradients: drawn by the card's own view in its first frame. */
function faceStyle(rarity: Rarity, disabled: boolean): ViewStyle {
  const key = `${rarity}:${disabled}`;
  const cached = faceStyles.get(key);
  if (cached) {
    return cached;
  }
  const { base, opacity, glows } = glowsFor(rarity, disabled);
  // CSS lists the top layer first: the second glow is drawn over the first.
  const layers = [...glows].reverse().map(glow => {
    const [from, to] = fade(glow.color, opacity);
    return (
      `radial-gradient(ellipse ${glow.r * 100}% ${glow.r * 100}% at ` +
      `${glow.cx * 100}% ${glow.cy * 100}%, ` +
      `${withAlpha(from.color, from.opacity)} 0%, ${withAlpha(
        to.color,
        to.opacity,
      )} 100%)`
    );
  });
  const style: ViewStyle = {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: base,
    experimental_backgroundImage: layers.join(', '),
  };
  faceStyles.set(key, style);
  return style;
}

/** `RarityBackground` drawn natively, and the card's rim, the same on every card, drawn once and
 * shown as one shared image. */
function CardFace({ rarity, disabled }: { rarity: Rarity; disabled: boolean }) {
  return (
    <>
      <View style={faceStyle(rarity, disabled)} />
      <CachedSvg cacheKey="card-rim" viewBox="0 0 96 96">
        <AngularPaths
          size={96}
          radius={5.94}
          width={1}
          stops={CARD_RIM}
          color={colors.snow}
          opacity={0.7}
        />
      </CachedSvg>
    </>
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
  /** Card width over 96 dp when the card is drawn smaller (slot rows, emote wheel): Figma scales
   * the whole instance, so radius, border, inset, plus, rings and shadow shrink with it. */
  scale?: number;
  /** `selected` only: the ring stroke in dp, centred on the edge, when the instance overrides it
   * (the Backpack slot rows draw 3 px). */
  ringWidth?: number;
};

const BAR_BG = {
  equip: colors.ruby,
  unequip: colors.shadow,
  detail: colors.shadow,
} as const;

/** Figma `Cards-Wearables` (96 dp square); the grid sizes it through `style`.
 * Figma Cards-Wearables: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=785-1018
 */
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
  scale = 1,
  ringWidth,
}: ItemCardProps) {
  const disabled = state === 'disabled';
  const k = scale;
  // Only the empty slot keeps a plain border; the others draw `AngularStroke` inside.
  const rim = empty ? k : 0;
  /** Values in dp at the 96 dp size; `ringWidth` is already final and drawn centred on the edge. */
  const ring = (width: number, outset: number, corner: number) => ({
    position: 'absolute' as const,
    top: -outset * k,
    left: -outset * k,
    right: -outset * k,
    bottom: -outset * k,
    borderWidth: width * k,
    borderRadius: corner * k,
  });
  const selectedRing = () => {
    const outset = empty ? 0 : OUTSET;
    const corner = empty ? radius.xl : SELECTED_CORNER;
    if (ringWidth === undefined) {
      return ring(SELECTED_WIDTH, outset, corner);
    }
    const half = ringWidth / 2;
    return {
      position: 'absolute' as const,
      top: -(outset * k + half),
      left: -(outset * k + half),
      right: -(outset * k + half),
      bottom: -(outset * k + half),
      borderWidth: ringWidth,
      borderRadius: corner * k + half,
    };
  };
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
        { borderRadius: radius.lg * k },
        !(empty && disabled) && (k === 1 ? elevation.down : shadow(k)),
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
                { borderRadius: radius.lg * k, borderWidth: rim },
                inset && {
                  top: INSET * k,
                  left: INSET * k,
                  right: INSET * k,
                  bottom: INSET * k,
                },
                empty
                  ? [
                      styles.emptyBg,
                      state === 'selected' && styles.emptySelectedBg,
                    ]
                  : disabled && styles.disabledBg,
              ]}
            >
              {empty ? (
                <Plus
                  scale={k}
                  opacity={state === 'selected' ? 0.5 : disabled ? 0.2 : 0.3}
                />
              ) : (
                <CardFace rarity={rarity} disabled={disabled} />
              )}
            </View>
            {!empty && (
              <View style={[styles.image, disabled && styles.dim]}>
                {thumbnail ? (
                  <Image
                    source={{ uri: thumbnail }}
                    // No fade-in (Android's default is 300 ms), and decoded at the tile's size: some
                    // thumbnails are 1024 px.
                    fadeDuration={0}
                    resizeMethod="resize"
                    style={styles.fill}
                    resizeMode="cover"
                  />
                ) : (
                  art ?? null
                )}
              </View>
            )}
            {tapped ? (
              <View
                style={[ring(THIN, 0, radius.xl), styles.ringLavenderColor]}
              />
            ) : state === 'equipped' ? (
              <View
                style={[
                  ring(THIN, OUTSET, radius.xl),
                  styles.ringLavenderColor,
                ]}
              />
            ) : action ? (
              <View
                style={[ring(u(2), OUTSET, radius.xl), styles.ringOrangeColor]}
              />
            ) : state === 'selected' ? (
              <View style={[selectedRing(), styles.ringOrangeColor]} />
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
            {price !== undefined && !action && !newLabel && (
              <View style={styles.priceRow} pointerEvents="none">
                <View style={styles.price}>
                  <Icon name="credits" size={13} />
                  <AppText style={styles.priceText}>{price}</AppText>
                </View>
              </View>
            )}
            {newLabel ? (
              <View
                style={[styles.newTag, tapped && styles.newTagInset]}
                pointerEvents="none"
              >
                <AppText style={styles.newText}>{newLabel}</AppText>
              </View>
            ) : null}
          </>
        );
      }}
    </Pressable>
  );
}

function Plus({ opacity, scale }: { opacity: number; scale: number }) {
  const long = u(16) * scale;
  const thick = u(2) * scale;
  return (
    <View style={[styles.plus, { opacity }]}>
      <View style={[styles.plusBar, { width: thick, height: long }]} />
      <View style={[styles.plusBar, { width: long, height: thick }]} />
    </View>
  );
}

const INSET = u(2.5);
/** `Rarity-Backgrounds` stroke: Snow, angular gradient with a highlight on each corner, at 70%. */
const CARD_RIM: AngularStop[] = [
  [0.12, 1],
  [0.25, 0],
  [0.38, 1],
  [0.5, 0],
  [0.62, 1],
  [0.75, 0],
  [0.88, 1],
  [1, 0],
];
const OUTSET = u(2.5);
const THIN = u(1.5);
const SELECTED_WIDTH = u(5);
const SELECTED_CORNER = u(9);
/** `Elevation/Down` scaled with the card. */
const shadow = (k: number) => ({
  boxShadow: [
    {
      offsetX: 0,
      offsetY: u(2) * k,
      blurRadius: u(2) * k,
      color: `${colors.shadow}80`,
    },
  ],
});

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
  emptyBg: { borderColor: colors.black, backgroundColor: colors.black40 },
  emptySelectedBg: { backgroundColor: colors.black60 },
  emptyDisabled: { opacity: 0.6 },
  disabledBg: { opacity: 0.8 },
  /** Proportional, so a smaller card (Card-Profile's 75 dp tile) keeps the 192 px layout. */
  image: {
    position: 'absolute',
    top: 0,
    left: '4.17%',
    right: '4.17%',
    bottom: '7.81%',
  },
  fill: { width: '100%', height: '100%' },
  dim: { opacity: 0.8 },
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
  /** Pressed: the tag moves in with the inset background. */
  newTagInset: { top: u(6.5), right: u(8) },
  newText: {
    fontFamily: fonts.bold,
    fontSize: u(9),
    color: colors.snow,
    textTransform: 'uppercase',
  },
  ringLavenderColor: { borderColor: colors.lavender },
  ringOrangeColor: { borderColor: colors.orange },
  plusBar: { position: 'absolute', backgroundColor: colors.snow },
  plus: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    alignItems: 'center',
    justifyContent: 'center',
  },
  skeleton: { backgroundColor: 'rgba(105,31,169,0.28)' },
});
