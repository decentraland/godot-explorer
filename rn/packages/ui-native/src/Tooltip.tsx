import { StyleProp, StyleSheet, View, ViewStyle } from 'react-native';
import Svg, { Path } from 'react-native-svg';
import { AppText } from './AppText';
import { colors, radius, space } from './tokens';
import { u } from './scale';

export type TooltipArrow = 'none' | 'left' | 'top' | 'right' | 'bottom';

// Purple 350 at 80%: the Figma fill has no alpha token of its own.
const FILL = `${colors.purple350}CC`;
// The outer half of an 18 px rounded diamond (3.46 corner), in dp.
const DEPTH = u(11.46);
const BASE = 2 * DEPTH;
const TIP_RADIUS = u(1.73);

type Point = [number, number];

function arrowPath(side: Exclude<TooltipArrow, 'none'>) {
  const [w, h] =
    side === 'left' || side === 'right' ? [DEPTH, BASE] : [BASE, DEPTH];
  const points: Record<typeof side, [Point, Point, Point]> = {
    left: [
      [w, 0],
      [0, h / 2],
      [w, h],
    ],
    right: [
      [0, 0],
      [w, h / 2],
      [0, h],
    ],
    top: [
      [0, h],
      [w / 2, 0],
      [w, h],
    ],
    bottom: [
      [0, 0],
      [w / 2, h],
      [w, 0],
    ],
  };
  const [a, tip, b] = points[side];
  const toward = (p: Point): Point => {
    const dx = p[0] - tip[0];
    const dy = p[1] - tip[1];
    const len = Math.hypot(dx, dy);
    return [tip[0] + (dx / len) * TIP_RADIUS, tip[1] + (dy / len) * TIP_RADIUS];
  };
  const [t1, t2] = [toward(a), toward(b)];
  return {
    w,
    h,
    d: `M${a} L${t1} Q${tip} ${t2} L${b} Z`,
  };
}

const ARROW_POSITION: Record<Exclude<TooltipArrow, 'none'>, ViewStyle> = {
  left: { left: -DEPTH, top: '50%', marginTop: -BASE / 2 },
  right: { right: -DEPTH, top: '50%', marginTop: -BASE / 2 },
  top: { top: -DEPTH, left: '50%', marginLeft: -BASE / 2 },
  bottom: { bottom: -DEPTH, left: '50%', marginLeft: -BASE / 2 },
};

/** Figma `Tooltip`: `arrow="none"` is the plain style for long text (`Tooltip/Long`, 1px border;
 * the Figma background blur is not drawn): it hugs the text, is at least 48 high, grows with lines
 * and wraps at 340. The other arrows point at a control and take one line of `Tooltip/Short`, cut
 * with an ellipsis at 340; longer text belongs in the plain style.
 * Figma Tooltip: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1163-18
 */
export function Tooltip({
  text,
  arrow = 'none',
  style,
}: {
  text: string;
  arrow?: TooltipArrow;
  style?: StyleProp<ViewStyle>;
}) {
  const plain = arrow === 'none';
  const shape =
    arrow === 'none'
      ? null
      : { ...arrowPath(arrow), position: ARROW_POSITION[arrow] };
  return (
    <View style={[styles.box, plain ? styles.plain : styles.pointed, style]}>
      {shape && (
        <Svg
          width={shape.w}
          height={shape.h}
          style={[styles.arrow, shape.position]}
        >
          <Path d={shape.d} fill={FILL} />
        </Svg>
      )}
      <AppText
        variant={plain ? 'Tooltip/Long' : 'Tooltip/Short'}
        numberOfLines={plain ? undefined : 1}
        style={!plain && styles.centred}
      >
        {text}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  box: {
    alignSelf: 'flex-start',
    maxWidth: u(170),
    minHeight: space['2xl'],
    justifyContent: 'center',
    borderRadius: radius.sm,
    backgroundColor: FILL,
  },
  plain: {
    paddingVertical: space.sm,
    paddingHorizontal: space['14'],
    borderWidth: u(0.5),
    borderColor: colors.purple350,
  },
  pointed: { paddingVertical: space.xs, paddingHorizontal: space.lg },
  arrow: { position: 'absolute' },
  centred: { textAlign: 'center' },
});
