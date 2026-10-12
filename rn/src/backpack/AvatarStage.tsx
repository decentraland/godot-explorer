import { ComponentRef, ReactNode, useCallback, useEffect, useRef } from 'react';
import {
  PixelRatio,
  StyleSheet,
  View,
  ViewStyle,
  StyleProp,
} from 'react-native';
import { colors, RadialFill } from '../ds';
import { send } from '../engine';

const REPORT_INTERVAL_MS = 50;

/** Purple stage the avatar stands on: radial glow plus a floor halo (Figma "Avatar background"). */
export function StageBackground() {
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <View
        style={[StyleSheet.absoluteFill, { backgroundColor: colors.purple500 }]}
      />
      <RadialFill
        cx={0.5}
        cy={0.42}
        rx={0.62}
        ry={0.5}
        stops={[
          { offset: 0, color: '#C13DFF' },
          { offset: 0.45, color: '#8A2BC4' },
          { offset: 1, color: '#4A1678' },
        ]}
      />
      <RadialFill
        cx={0.5}
        cy={0.97}
        rx={0.55}
        ry={0.14}
        stops={[
          { offset: 0, color: '#B659E8', opacity: 0.85 },
          { offset: 1, color: '#B659E8', opacity: 0 },
        ]}
      />
    </View>
  );
}

/**
 * Transparent window Godot draws the 3D avatar into. The rectangle is reported to Godot in device
 * pixels whenever it changes (sheet drag, rotation); closes the preview when it unmounts.
 */
export function AvatarHole({
  style,
  children,
}: {
  style?: StyleProp<ViewStyle>;
  children?: ReactNode;
}) {
  const ref = useRef<ComponentRef<typeof View>>(null);
  const opened = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const report = useCallback(() => {
    ref.current?.measureInWindow((x, y, w, h) => {
      if (w <= 0 || h <= 0) {
        return;
      }
      const scale = PixelRatio.get();
      const type = opened.current ? 'backpack_place' : 'backpack_open';
      opened.current = true;
      send({ type, x: x * scale, y: y * scale, w: w * scale, h: h * scale });
    });
  }, []);

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
