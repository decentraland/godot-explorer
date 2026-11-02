import type { ReactElement } from 'react';
import { StyleSheet, View } from 'react-native';
import Svg, { Circle, Ellipse, G, Line, Path } from 'react-native-svg';

const TONES = [
  ['#3f8f86', '#256862'],
  ['#8a9a94', '#5f6f6a'],
  ['#9a6a2e', '#6e4519'],
  ['#e8a92c', '#a85f10'],
  ['#4b86d6', '#2a58a3'],
  ['#f1eef5', '#b7aec4'],
  ['#c4506e', '#8c2f4b'],
];

const hash = (text: string) =>
  [...text].reduce((sum, ch) => (sum * 31 + ch.charCodeAt(0)) % 1000003, 7);

const SKIN = '#f2c9a5';

type Shape = (main: string, dark: string) => ReactElement;

const SHAPES: Record<string, Shape> = {
  upper_body: (c, d) => (
    <>
      <Path
        d="M30 22 L42 16 Q50 26 58 16 L70 22 L88 56 L76 62 L70 48 L70 86 L30 86 L30 48 L24 62 L12 56 Z"
        fill={c}
      />
      <Path d="M42 16 Q50 6 58 16 Q50 30 42 16Z" fill={d} />
      <Line x1="50" y1="28" x2="50" y2="86" stroke={d} strokeWidth="2" />
      <Path
        d="M12 56 L24 62 L22 68 L10 62Z M88 56 L76 62 L78 68 L90 62Z"
        fill={d}
      />
    </>
  ),
  lower_body: (c, d) => (
    <>
      <Path d="M30 12 H70 L76 90 H56 L50 40 L44 90 H24 Z" fill={c} />
      <Path d="M30 12 H70 V20 H30Z" fill={d} />
    </>
  ),
  feet: (c, d) => (
    <>
      <Path d="M10 66 Q10 48 28 52 L44 64 V78 H10Z" fill={c} />
      <Path d="M90 66 Q90 48 72 52 L56 64 V78 H90Z" fill={c} />
      <Path d="M10 74 H44 V80 H10Z M56 74 H90 V80 H56Z" fill={d} />
    </>
  ),
  hands_wear: (c, d) => (
    <>
      <Path
        d="M32 90 V58 L24 42 Q22 34 30 36 L38 48 V22 Q38 14 45 14 Q52 14 52 22 V40 V18 Q52 10 59 10 Q66 10 66 18 V44 V26 Q66 20 72 22 Q76 26 74 32 V62 L68 90Z"
        fill={c}
      />
      <Path d="M32 80 H68 V90 H32Z" fill={d} />
    </>
  ),
  hat: (c, d) => (
    <>
      <Path d="M18 64 Q18 24 50 24 Q82 24 82 64Z" fill={c} />
      <Path d="M8 64 H92 Q94 76 80 76 H20 Q6 76 8 64Z" fill={d} />
    </>
  ),
  helmet: (c, d) => (
    <>
      <Path
        d="M18 72 Q12 22 50 18 Q88 22 82 72 Q82 80 74 80 H26 Q18 80 18 72Z"
        fill={c}
      />
      <Path d="M28 46 H72 V64 H28Z" fill={d} />
    </>
  ),
  tiara: (c, d) => (
    <>
      <Path d="M12 72 L20 32 L36 54 L50 24 L64 54 L80 32 L88 72Z" fill={c} />
      <Circle cx="50" cy="40" r="5" fill={d} />
    </>
  ),
  eyewear: (c, d) => (
    <>
      <Circle cx="30" cy="52" r="17" fill="none" stroke={c} strokeWidth="7" />
      <Circle cx="70" cy="52" r="17" fill="none" stroke={c} strokeWidth="7" />
      <Line x1="46" y1="50" x2="54" y2="50" stroke={d} strokeWidth="5" />
    </>
  ),
  mask: (c, d) => (
    <>
      <Path d="M14 38 H86 Q90 70 50 86 Q10 70 14 38Z" fill={c} />
      <Path d="M30 54 H70 M32 64 H68" stroke={d} strokeWidth="3" />
    </>
  ),
  earring: (c, d) => (
    <>
      <G>
        <Circle cx="30" cy="24" r="6" fill={d} />
        <Path d="M30 32 L40 52 L30 76 L20 52Z" fill={c} />
        <Circle cx="70" cy="24" r="6" fill={d} />
        <Path d="M70 32 L80 52 L70 76 L60 52Z" fill={c} />
      </G>
    </>
  ),
  hair: (c, d) => (
    <>
      <Path
        d="M18 66 Q10 16 50 14 Q90 16 82 66 Q72 38 50 40 Q28 38 18 66Z"
        fill={c}
      />
      <Path d="M30 30 Q50 20 70 30" stroke={d} strokeWidth="3" fill="none" />
    </>
  ),
  facial_hair: (c, d) => (
    <>
      <Path
        d="M22 34 Q22 90 50 90 Q78 90 78 34 Q68 62 50 58 Q32 62 22 34Z"
        fill={c}
      />
      <Path d="M40 68 Q50 74 60 68" stroke={d} strokeWidth="3" fill="none" />
    </>
  ),
  eyes: (c, d) => (
    <>
      <Ellipse cx="50" cy="50" rx="34" ry="20" fill="#fff" />
      <Circle cx="50" cy="50" r="15" fill={c} />
      <Circle cx="50" cy="50" r="7" fill={d} />
    </>
  ),
  eyebrows: (c, d) => (
    <>
      <Path d="M10 56 Q26 34 44 50 L42 60 Q28 48 14 64Z" fill={c} />
      <Path d="M90 56 Q74 34 56 50 L58 60 Q72 48 86 64Z" fill={d} />
    </>
  ),
  mouth: () => (
    <>
      <Path d="M18 50 Q50 34 82 50 Q50 80 18 50Z" fill="#c4506e" />
      <Path
        d="M18 50 Q50 58 82 50"
        stroke="#7a2038"
        strokeWidth="3"
        fill="none"
      />
    </>
  ),
  body_shape: () => <Figure color="#3a2a52" skin />,
  skin: (c, d) => <Figure color={c} accent={d} />,
};

function Figure({
  color,
  accent = color,
  skin = false,
}: {
  color: string;
  accent?: string;
  skin?: boolean;
}) {
  return (
    <>
      <Circle cx="50" cy="18" r="11" fill={skin ? SKIN : accent} />
      <Path d="M32 34 H68 L74 66 H26Z" fill={color} />
      <Path d="M32 66 H48 L46 94 H34Z M52 66 H68 L66 94 H54Z" fill={accent} />
    </>
  );
}

/** Stand-in artwork for an item whose thumbnail is missing, drawn from its category and name. */
export function ItemArt({
  category,
  name,
}: {
  category: string;
  name: string;
}) {
  const [main, dark] = TONES[hash(name) % TONES.length];
  const draw = SHAPES[category] ?? SHAPES.upper_body;
  return (
    <View pointerEvents="none" style={styles.fill}>
      <Svg width="100%" height="100%" viewBox="0 0 100 100">
        <G scale={0.82} origin="50, 50">
          {draw(main, dark)}
        </G>
      </Svg>
    </View>
  );
}

type Pose = { la: number; ra: number; ll: number; rl: number };
/** Angles in degrees from hanging straight down, swinging outwards. */
const POSES: Pose[] = [
  { la: 150, ra: 40, ll: 6, rl: 6 },
  { la: 170, ra: 20, ll: 8, rl: 24 },
  { la: 70, ra: 70, ll: 8, rl: 8 },
  { la: 25, ra: 165, ll: 5, rl: 14 },
  { la: 95, ra: 95, ll: 18, rl: 18 },
  { la: 140, ra: 140, ll: 12, rl: 12 },
  { la: 60, ra: 120, ll: 22, rl: 4 },
];

const limb = (
  x: number,
  y: number,
  deg: number,
  length: number,
  side: 1 | -1,
) => {
  const rad = (deg * Math.PI) / 180;
  return {
    x2: x + side * Math.sin(rad) * length,
    y2: y + Math.cos(rad) * length,
  };
};

/** Black dancer silhouette standing in for an emote thumbnail; the pose follows the emote id. */
export function EmoteArt({ id }: { id: string }) {
  const pose = POSES[hash(id) % POSES.length];
  const parts = [
    { x: 43, y: 34, deg: pose.la, len: 23, w: 6, side: -1 as const },
    { x: 57, y: 34, deg: pose.ra, len: 23, w: 6, side: 1 as const },
    { x: 46, y: 62, deg: pose.ll, len: 30, w: 8, side: -1 as const },
    { x: 54, y: 62, deg: pose.rl, len: 30, w: 8, side: 1 as const },
  ];
  return (
    <View pointerEvents="none" style={styles.fill}>
      <Svg width="100%" height="100%" viewBox="0 0 100 100">
        <Circle cx="50" cy="17" r="8" fill="#111" />
        <Line
          x1="50"
          y1="28"
          x2="50"
          y2="62"
          stroke="#111"
          strokeWidth="13"
          strokeLinecap="round"
        />
        {parts.map(({ x, y, deg, len, w, side }) => (
          <Line
            key={`${x}-${y}`}
            x1={x}
            y1={y}
            {...limb(x, y, deg, len, side)}
            stroke="#111"
            strokeWidth={w}
            strokeLinecap="round"
          />
        ))}
      </Svg>
    </View>
  );
}

const styles = StyleSheet.create({
  fill: { ...StyleSheet.absoluteFill, padding: '8%' },
});
