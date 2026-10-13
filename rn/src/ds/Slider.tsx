import { useMemo, useRef, useState } from 'react';
import { PanResponder, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { colors } from './tokens';
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

/** Ruby-fill slider with a white pill thumb and a right-aligned "NN%" value. */
export function Slider({
  label,
  value,
  onChange,
  onChangeEnd,
  min = 0,
  max = 100,
  step = 1,
  disabled = false,
  variant = 'default',
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  onChangeEnd?: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  /** `settings`: square-cornered bar, white body-size value, roomier label row. */
  variant?: 'default' | 'settings';
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

  const flat = variant === 'settings';
  const ratio = max > min ? (value - min) / (max - min) : 0;
  const thumbLeft = ratio * Math.max(0, width - THUMB_W);

  return (
    <View style={[styles.root, disabled && styles.disabled]}>
      <View style={[styles.row, flat && styles.rowFlat]}>
        <AppText variant="body" style={styles.label}>
          {label}
        </AppText>
        <AppText
          variant={flat ? 'body' : 'caption'}
          style={flat ? styles.valueFlat : styles.value}
        >{`${value}%`}</AppText>
      </View>
      <View
        accessible
        accessibilityRole="adjustable"
        accessibilityLabel={label}
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
        style={[styles.track, flat && styles.trackFlat]}
        {...responder.panHandlers}
      >
        <View
          style={[
            styles.fill,
            flat && styles.trackFlat,
            { width: thumbLeft + THUMB_W / 2 },
          ]}
        />
        <View
          style={[styles.thumb, flat && styles.thumbFlat, { left: thumbLeft }]}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'stretch' },
  disabled: { opacity: 0.35 },
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: u(8),
  },
  label: { fontSize: u(15) },
  rowFlat: { marginBottom: u(12) },
  value: { color: colors.lavender },
  valueFlat: { fontSize: u(15), color: colors.snow },
  trackFlat: { borderRadius: u(3) },
  thumbFlat: { borderRadius: u(3) },
  track: {
    height: u(20),
    borderRadius: u(10),
    backgroundColor: colors.fieldDark,
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
  thumb: {
    position: 'absolute',
    top: -u(2.5),
    width: THUMB_W,
    height: u(25),
    borderRadius: u(8),
    backgroundColor: colors.snow,
  },
});
