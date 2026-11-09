import { useId, useState } from 'react';
import { StyleSheet, useWindowDimensions, View } from 'react-native';
import Svg, { Defs, RadialGradient, Rect, Stop } from 'react-native-svg';
import { colors, rarityColors } from './tokens';

type Glow = {
  color: string;
  /** Figma's `gradientTransform` on the 720×1600 (or 1600×720) frame, r = 10. */
  matrix: [number, number, number, number, number, number];
  stops: [number, number][];
};

const legendary = rarityColors.legendary.dark;
const mythic = rarityColors.mythic.dark;

const PORTRAIT: Glow[] = [
  {
    color: legendary,
    matrix: [-66.394, 41.477, 2.5892, -64.277, 157.31, 80.813],
    stops: [
      [0, 0.4985],
      [0.15857, 0.4886],
      [0.36893, 0.4207],
      [0.57928, 0.25],
      [0.78964, 0.0793],
      [1, 0],
    ],
  },
  {
    color: mythic,
    matrix: [-47.622, 40.344, -35.006, -41.321, 647.35, 1423.8],
    stops: [
      [0, 0.2911],
      [0.22793, 0.2524],
      [0.48529, 0.15],
      [0.74264, 0.0476],
      [1, 0],
    ],
  },
];
const LANDSCAPE: Glow[] = [
  {
    color: legendary,
    matrix: [-92.055, 25.879, 4.8535, -54.222, 244.72, -149.16],
    stops: [
      [0, 0.4999],
      [0.33361, 0.4886],
      [0.53353, 0.4207],
      [0.73344, 0.25],
      [0.86672, 0.1262],
      [1, 0],
    ],
  },
];

/** Figma `Background` (Backgrounds page): `background/brand` alone (`Plain`) or with the radial
 * glows (`Glow`: Legendary top-left, plus Mythic bottom-right in portrait). Fills its parent. */
export function BrandBackground({ glow = false }: { glow?: boolean }) {
  const window = useWindowDimensions();
  const [size, setSize] = useState({ width: 0, height: 0 });
  const id = useId();
  const landscape = window.width > window.height;
  const glows = landscape ? LANDSCAPE : PORTRAIT;
  const sx = size.width / (landscape ? 1600 : 720);
  const sy = size.height / (landscape ? 720 : 1600);
  return (
    <View
      pointerEvents="none"
      style={[StyleSheet.absoluteFill, { backgroundColor: colors.brand }]}
      onLayout={event => setSize(event.nativeEvent.layout)}
    >
      {glow && size.width > 0 && (
        <Svg width={size.width} height={size.height}>
          <Defs>
            {glows.map(({ color, matrix: [a, b, c, d, e, f], stops }, i) => (
              <RadialGradient
                key={i}
                id={`${id}-${i}`}
                cx={0}
                cy={0}
                r={10}
                gradientUnits="userSpaceOnUse"
                gradientTransform={`matrix(${sx * a} ${sy * b} ${sx * c} ${
                  sy * d
                } ${sx * e} ${sy * f})`}
              >
                {stops.map(([offset, opacity]) => (
                  <Stop
                    key={offset}
                    offset={offset}
                    stopColor={color}
                    stopOpacity={opacity}
                  />
                ))}
              </RadialGradient>
            ))}
          </Defs>
          {glows.map((_, i) => (
            <Rect
              key={i}
              width={size.width}
              height={size.height}
              fill={`url(#${id}-${i})`}
            />
          ))}
        </Svg>
      )}
    </View>
  );
}
