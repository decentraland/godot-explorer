import { useMemo } from 'react';
import { StyleSheet } from 'react-native';
import Svg, { Path } from 'react-native-svg';

/** Opacity stop of a Figma `GRADIENT_ANGULAR` paint: position 0..1 along the sweep. */
export type AngularStop = [position: number, alpha: number];

/** Figma's angular-gradient `gradientTransform` rows [[a, b, c], [d, e, f]]. */
type Transform = [[number, number, number], [number, number, number]];

/** The transform every angular stroke in the design file uses: the sweep starts at the bottom
 * centre and turns clockwise. */
const BOTTOM_CLOCKWISE: Transform = [
  [0, 1, 0],
  [-1, 0, 1],
];
const SEGMENTS = 96;

/**
 * A rounded-rect (or circle) stroke whose opacity follows a Figma angular gradient. React Native has
 * no conic gradient, so the outline is split into short SVG segments, each drawn at the opacity
 * the gradient has at its midpoint. `size`, `radius` and `width` are in the node's own units (the
 * Figma frame / 2), drawn inside the edge; the SVG scales to its parent.
 */
export function AngularStroke({ size, ...rest }: AngularPathsProps) {
  return (
    <Svg
      style={StyleSheet.absoluteFill}
      viewBox={`0 0 ${size} ${size}`}
      pointerEvents="none"
    >
      <AngularPaths size={size} {...rest} />
    </Svg>
  );
}

type AngularPathsProps = {
  size: number;
  radius: number;
  width: number;
  stops: AngularStop[];
  color: string;
  opacity?: number;
  transform?: Transform;
};

/** The segments of `AngularStroke`, for drawing inside another SVG with the same `size` viewBox. */
export function AngularPaths({
  size,
  radius,
  width,
  stops,
  color,
  opacity = 1,
  transform = BOTTOM_CLOCKWISE,
}: AngularPathsProps) {
  const segments = useMemo(
    () => outline(size, radius, width, stops, transform),
    [size, radius, width, stops, transform],
  );
  return (
    <>
      {segments.map(([d, alpha], i) =>
        alpha > 0.001 ? (
          <Path
            key={i}
            d={d}
            fill="none"
            stroke={color}
            strokeWidth={width}
            strokeOpacity={alpha * opacity}
          />
        ) : null,
      )}
    </>
  );
}

function outline(
  size: number,
  radius: number,
  width: number,
  stops: AngularStop[],
  [[a, b, c], [d, e, f]]: Transform,
): [string, number][] {
  const half = width / 2;
  const side = size - width;
  const r = Math.max(0, Math.min(radius - half, side / 2));
  const straight = side - 2 * r;
  const arc = (Math.PI / 2) * r;
  const length = 4 * straight + 4 * arc;
  // Walk the centre line clockwise from the top edge's left end.
  const point = (s: number): [number, number] => {
    const lo = half;
    const hi = half + side;
    const legs: [number, (t: number) => [number, number]][] = [
      [straight, t => [lo + r + t, lo]],
      [arc, t => corner(hi - r, lo + r, -90, t)],
      [straight, t => [hi, lo + r + t]],
      [arc, t => corner(hi - r, hi - r, 0, t)],
      [straight, t => [hi - r - t, hi]],
      [arc, t => corner(lo + r, hi - r, 90, t)],
      [straight, t => [lo, hi - r - t]],
      [arc, t => corner(lo + r, lo + r, 180, t)],
    ];
    let rest = ((s % length) + length) % length;
    for (const [len, at] of legs) {
      if (rest <= len) {
        return at(rest);
      }
      rest -= len;
    }
    return [lo + r, lo];
  };
  const corner = (cx: number, cy: number, from: number, t: number) => {
    const angle =
      ((from + (r > 0 ? (t / r) * (180 / Math.PI) : 0)) * Math.PI) / 180;
    return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)] as [
      number,
      number,
    ];
  };
  const alphaAt = (x: number, y: number) => {
    const u = x / size;
    const v = y / size;
    const gx = a * u + b * v + c;
    const gy = d * u + e * v + f;
    const turn = Math.atan2(gy - 0.5, gx - 0.5) / (2 * Math.PI);
    return interpolate(stops, ((turn % 1) + 1) % 1);
  };
  const step = length / SEGMENTS;
  return Array.from({ length: SEGMENTS }, (_, i) => {
    const s0 = i * step;
    const [x0, y0] = point(s0);
    const [xm, ym] = point(s0 + step / 2);
    const [x1, y1] = point(s0 + step);
    return [`M${x0} ${y0}L${xm} ${ym}L${x1} ${y1}`, alphaAt(xm, ym)];
  });
}

/** Figma's sweep wraps: past the last stop it blends back into the first one. */
function interpolate(stops: AngularStop[], t: number) {
  const ring = [
    [stops[stops.length - 1][0] - 1, stops[stops.length - 1][1]],
    ...stops,
    [stops[0][0] + 1, stops[0][1]],
  ];
  for (let i = 1; i < ring.length; i++) {
    const [p1, a1] = ring[i];
    if (t <= p1) {
      const [p0, a0] = ring[i - 1];
      return a0 + ((a1 - a0) * (t - p0)) / (p1 - p0 || 1);
    }
  }
  return stops[stops.length - 1][1];
}
