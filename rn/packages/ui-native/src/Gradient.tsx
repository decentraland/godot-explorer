import { ReactNode, useId, useState } from 'react';
import { StyleProp, StyleSheet, View, ViewStyle } from 'react-native';
import Svg, {
  Defs,
  LinearGradient,
  RadialGradient,
  Rect,
  Stop,
} from 'react-native-svg';

/** Hands children the box's own size: percentage-sized SVGs kept a stale height after rotation. */
function Measured({
  children,
}: {
  children: (width: number, height: number) => ReactNode;
}) {
  const [size, setSize] = useState({ width: 0, height: 0 });
  return (
    <View
      style={StyleSheet.absoluteFill}
      pointerEvents="none"
      onLayout={event => setSize(event.nativeEvent.layout)}
    >
      {size.width > 0 && size.height > 0
        ? children(size.width, size.height)
        : null}
    </View>
  );
}

export type GradientStop = { offset: number; color: string; opacity?: number };

/** Absolute-fill linear gradient; `angle` 0 = top→bottom, 90 = left→right. */
export function LinearFill({
  stops,
  angle = 0,
}: {
  stops: GradientStop[];
  angle?: number;
}) {
  const id = useId();
  const rad = (angle * Math.PI) / 180;
  const dx = Math.sin(rad) / 2;
  const dy = Math.cos(rad) / 2;
  return (
    <Measured>
      {(width, height) => (
        <Svg width={width} height={height}>
          <Defs>
            <LinearGradient
              id={id}
              x1={0.5 - dx}
              y1={0.5 - dy}
              x2={0.5 + dx}
              y2={0.5 + dy}
            >
              {stops.map(stop => (
                <Stop
                  key={stop.offset}
                  offset={stop.offset}
                  stopColor={stop.color}
                  stopOpacity={stop.opacity ?? 1}
                />
              ))}
            </LinearGradient>
          </Defs>
          <Rect width={width} height={height} fill={`url(#${id})`} />
        </Svg>
      )}
    </Measured>
  );
}

/** Absolute-fill radial gradient centred at (cx, cy) as fractions of the box. */
export function RadialFill({
  stops,
  cx = 0.5,
  cy = 0.5,
  rx = 0.5,
  ry = 0.5,
}: {
  stops: GradientStop[];
  cx?: number;
  cy?: number;
  rx?: number;
  ry?: number;
}) {
  const id = useId();
  return (
    <Measured>
      {(width, height) => (
        <Svg width={width} height={height}>
          <Defs>
            <RadialGradient
              id={id}
              cx={cx}
              cy={cy}
              rx={rx}
              ry={ry}
              fx={cx}
              fy={cy}
            >
              {stops.map(stop => (
                <Stop
                  key={stop.offset}
                  offset={stop.offset}
                  stopColor={stop.color}
                  stopOpacity={stop.opacity ?? 1}
                />
              ))}
            </RadialGradient>
          </Defs>
          <Rect width={width} height={height} fill={`url(#${id})`} />
        </Svg>
      )}
    </Measured>
  );
}

export function GradientView({
  stops,
  angle,
  style,
  children,
}: {
  stops: GradientStop[];
  angle?: number;
  style?: StyleProp<ViewStyle>;
  children?: ReactNode;
}) {
  return (
    <View style={[style, styles.clip]}>
      <LinearFill stops={stops} angle={angle} />
      {children}
    </View>
  );
}

const styles = StyleSheet.create({ clip: { overflow: 'hidden' } });
