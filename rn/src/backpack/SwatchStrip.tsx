import { ReactNode } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import Svg, {
  Circle,
  Defs,
  Path,
  RadialGradient,
  Stop,
} from 'react-native-svg';
import { colors, layout } from '../ds';
import { hsvToHex } from '../ds/colorMath';
import { t } from '../i18n';
import { u } from '../ds/scale';

const SWATCH = 40;
const RING = 44;
const WEDGES = 12;

const wedge = (index: number) => {
  const a0 = (index / WEDGES) * 2 * Math.PI;
  const a1 = ((index + 1) / WEDGES) * 2 * Math.PI;
  const point = (a: number) =>
    `${50 + 50 * Math.sin(a)} ${50 - 50 * Math.cos(a)}`;
  return `M50 50 L${point(a0)} A50 50 0 0 1 ${point(a1)} Z`;
};
const WEDGE_PATHS = Array.from({ length: WEDGES }, (_, i) => ({
  d: wedge(i),
  fill: hsvToHex({ h: (i + 0.5) * (360 / WEDGES), s: 1, v: 1 }),
}));

/** Rainbow colour wheel: hue wedges fading to white in the middle. */
export function RainbowWheel({ size }: { size: number }) {
  return (
    <Svg width={size} height={size} viewBox="0 0 100 100">
      <Defs>
        <RadialGradient
          id="fade"
          cx="50"
          cy="50"
          r="50"
          gradientUnits="userSpaceOnUse"
        >
          <Stop offset="0" stopColor="#ffffff" stopOpacity={1} />
          <Stop offset="0.7" stopColor="#ffffff" stopOpacity={0} />
        </RadialGradient>
      </Defs>
      {WEDGE_PATHS.map(wedgePath => (
        <Path key={wedgePath.d} d={wedgePath.d} fill={wedgePath.fill} />
      ))}
      <Circle cx="50" cy="50" r="50" fill="url(#fade)" />
    </Svg>
  );
}

function Dot({
  selected,
  disabled,
  label,
  onPress,
  children,
}: {
  selected: boolean;
  disabled: boolean;
  label: string;
  onPress: () => void;
  children: ReactNode;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected, disabled }}
      disabled={disabled}
      onPress={onPress}
      style={[styles.cell, selected && styles.cellOn]}
    >
      {children}
    </Pressable>
  );
}

/** Row of colour swatches; the rainbow wheel opens the editor and is ringed when the colour is custom. */
export function SwatchStrip({
  palette,
  value,
  custom,
  disabled = false,
  onSelect,
  onWheel,
}: {
  palette: string[];
  value: string;
  /** True when `value` is not one of the presets: the wheel gets the ring. */
  custom: boolean;
  disabled?: boolean;
  onSelect: (color: string) => void;
  /** Omit it for the editor's own row, which has no wheel. */
  onWheel?: () => void;
}) {
  const selected = (color: string) =>
    !custom && color.toLowerCase() === value.toLowerCase();
  return (
    <ScrollView
      removeClippedSubviews={false}
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={[styles.row, disabled && styles.disabled]}
    >
      {onWheel ? (
        <Dot
          selected={custom}
          disabled={disabled}
          label={t('BACKPACK_CUSTOM_COLOR')}
          onPress={onWheel}
        >
          <RainbowWheel size={u(custom ? 34 : SWATCH)} />
        </Dot>
      ) : null}
      {palette.map(color => (
        <Dot
          key={color}
          selected={selected(color)}
          disabled={disabled}
          label={color}
          onPress={() => onSelect(color)}
        >
          <View
            style={[
              styles.fill,
              { backgroundColor: color },
              selected(color) && styles.fillOn,
            ]}
          />
        </Dot>
      ))}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  row: {
    paddingHorizontal: layout.gutter - u((RING - SWATCH) / 2),
    gap: u(11),
    alignItems: 'center',
  },
  disabled: { opacity: 0.4 },
  cell: {
    width: u(RING),
    height: u(RING),
    borderRadius: u(RING / 2),
    borderWidth: u(1.5),
    borderColor: 'transparent',
    alignItems: 'center',
    justifyContent: 'center',
  },
  cellOn: { borderColor: colors.ruby },
  fill: {
    width: u(SWATCH),
    height: u(SWATCH),
    borderRadius: u(SWATCH / 2),
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.45)',
  },
  fillOn: {
    width: u(SWATCH - 6),
    height: u(SWATCH - 6),
    borderRadius: u((SWATCH - 6) / 2),
  },
});
