import { ComponentRef, ReactNode, useCallback, useEffect, useRef } from 'react';
import {
  PixelRatio,
  StyleSheet,
  View,
  ViewStyle,
  StyleProp,
} from 'react-native';
import { colors, RadialFill } from '../ds';
import type { CameraTarget } from '../GodotBridge';
import { send } from '../engine';

const REPORT_INTERVAL_MS = 50;

/** Purple stage: a glow centred on the avatar window (everything above `bottom`) and a floor halo at its feet. */
export function StageBackground({ bottom = 0 }: { bottom?: number }) {
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <View
        style={[StyleSheet.absoluteFill, { backgroundColor: colors.purple500 }]}
      />
      <View style={[StyleSheet.absoluteFill, { bottom }]}>
        <RadialFill
          cx={0.5}
          cy={0.5}
          rx={0.7}
          ry={0.62}
          stops={[
            { offset: 0, color: '#C040F5' },
            { offset: 0.3, color: '#9430C3' },
            { offset: 0.65, color: '#5F1E83' },
            { offset: 1, color: '#3C1859' },
          ]}
        />
        <RadialFill
          cx={0.5}
          cy={0.94}
          rx={0.5}
          ry={0.08}
          stops={[
            { offset: 0, color: '#B659E8', opacity: 0.85 },
            { offset: 1, color: '#B659E8', opacity: 0 },
          ]}
        />
      </View>
    </View>
  );
}

/**
 * Transparent window Godot draws the 3D avatar into. The rectangle is reported to Godot in device
 * pixels whenever it changes (sheet drag, rotation); closes the preview when it unmounts.
 */
export function AvatarHole({
  style,
  focus,
  children,
}: {
  style?: StyleProp<ViewStyle>;
  /** Camera framing applied once the preview exists. */
  focus?: CameraTarget;
  children?: ReactNode;
}) {
  const ref = useRef<ComponentRef<typeof View>>(null);
  const opened = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const alive = useRef(true);

  const report = useCallback(() => {
    ref.current?.measureInWindow((x, y, w, h) => {
      if (!alive.current || w <= 0 || h <= 0) {
        return;
      }
      const scale = PixelRatio.get();
      const first = !opened.current;
      const type = first ? 'backpack_open' : 'backpack_place';
      opened.current = true;
      send({ type, x: x * scale, y: y * scale, w: w * scale, h: h * scale });
      if (first && focus) {
        send({ type: 'focus_camera', target: focus });
      }
    });
  }, [focus]);

  const schedule = useCallback(() => {
    if (timer.current) {
      return;
    }
    timer.current = setTimeout(() => {
      timer.current = null;
      report();
    }, REPORT_INTERVAL_MS);
  }, [report]);

  useEffect(
    () => () => {
      alive.current = false;
      if (timer.current) {
        clearTimeout(timer.current);
      }
      send({ type: 'backpack_close' });
    },
    [],
  );

  return (
    <View ref={ref} style={style} pointerEvents="none" onLayout={schedule}>
      {children}
    </View>
  );
}
