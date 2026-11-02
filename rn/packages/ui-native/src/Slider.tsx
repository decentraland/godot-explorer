import { useMemo, useRef, useState } from 'react';
import { PanResponder, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { colors, elevation, fonts } from './tokens';
import { u } from './scale';

/** Maps a touch x inside a track of `width` to a clamped, stepped value. */
export function sliderValueFromX(
  x: number,
  width: number,
  min = 0,
  max = 100,
  step = 1,
): number {
  if (width <= 0) {
    return min;
  }
  const ratio = Math.min(1, Math.max(0, x / width));
  const raw = min + ratio * (max - min);
  return Math.min(max, Math.max(min, Math.round(raw / step) * step));
}

const THUMB_W = u(16);

/** Figma `Control/Slider` under an `Input/Label` Value row: Black 40% track, Ruby fill, Snow thumb
 * with `Elevation/Down`; disabled swaps both for Pale Black instead of dimming. */
export function Slider({
  label,
  value,
  onChange,
  onChangeEnd,
  min = 0,
  max = 100,
  step = 1,
  disabled = false,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  onChangeEnd?: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
}) {
  const [width, setWidth] = useState(0);
  const latest = useRef({ width, onChange, onChangeEnd, min, max, step });
  latest.current = { width, onChange, onChangeEnd, min, max, step };
  const last = useRef(value);
  const trackRef = useRef<View>(null);
  const pageX = useRef(0);

  const measure = () =>
    trackRef.current?.measure((_x, _y, _w, _h, px) => {
      pageX.current = px;
    });

  const responder = useMemo(() => {
    const at = (x: number) => {
      const c = latest.current;
      return sliderValueFromX(x, c.width, c.min, c.max, c.step);
    };
    const move = (x: number) => {
      last.current = at(x);
      latest.current.onChange(last.current);
    };
    return PanResponder.create({
      onStartShouldSetPanResponder: () => !disabled,
      onMoveShouldSetPanResponder: () => !disabled,
      onPanResponderTerminationRequest: () => false,
      onPanResponderGrant: (_, g) => {
        move(g.x0 - pageX.current);
      },
      onPanResponderMove: (_, g) => move(g.moveX - pageX.current),
      onPanResponderRelease: () => latest.current.onChangeEnd?.(last.current),
    });
  }, [disabled]);

  const ratio = max > min ? (value - min) / (max - min) : 0;
  const thumbLeft = ratio * Math.max(0, width - THUMB_W);

  return (
    <View style={styles.root}>
      <View style={styles.row}>
        <AppText style={[styles.label, disabled && styles.textDisabled]}>
          {label}
        </AppText>
        <AppText style={[styles.value, disabled && styles.textDisabled]}>
          {`${value}%`}
        </AppText>
      </View>
      <View
        accessible
        accessibilityRole="adjustable"
        accessibilityLabel={label}
        accessibilityState={{ disabled }}
        accessibilityValue={{ min, max, now: value }}
        accessibilityActions={[{ name: 'increment' }, { name: 'decrement' }]}
        onAccessibilityAction={e => {
          const delta = e.nativeEvent.actionName === 'increment' ? step : -step;
          if (!disabled) {
            onChange(Math.min(max, Math.max(min, value + delta)));
          }
        }}
        onLayout={e => {
          setWidth(e.nativeEvent.layout.width);
          measure();
        }}
        ref={trackRef}
        style={styles.track}
        {...responder.panHandlers}
      >
        <View
          style={[
            styles.fill,
            disabled && styles.fillDisabled,
            { width: thumbLeft + THUMB_W / 2 },
          ]}
        />
        <View
          style={[
            styles.thumb,
            disabled && styles.thumbDisabled,
            { left: thumbLeft },
          ]}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'stretch' },
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: u(10),
  },
  label: { fontFamily: fonts.medium, fontSize: u(15), color: colors.snow },
  value: { fontSize: u(15), color: colors.snow },
  textDisabled: { color: colors.white30 },
  track: {
    height: u(20),
    borderRadius: u(10),
    backgroundColor: colors.black40,
    justifyContent: 'center',
  },
  fill: {
    position: 'absolute',
    left: 0,
    top: 0,
    bottom: 0,
    borderRadius: u(10),
    backgroundColor: colors.ruby,
  },
  fillDisabled: { backgroundColor: colors.paleBlack, opacity: 0.8 },
  thumb: {
    position: 'absolute',
    top: -u(2.5),
    width: THUMB_W,
    height: u(25),
    borderRadius: u(4),
    borderWidth: 1,
    borderColor: colors.pearl,
    backgroundColor: colors.snow,
    ...elevation.down,
  },
  thumbDisabled: { backgroundColor: colors.paleBlack, borderWidth: 0 },
});
