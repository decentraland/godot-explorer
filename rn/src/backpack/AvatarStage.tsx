import { ComponentRef, ReactNode, useCallback, useEffect, useRef } from 'react';
import {
  Image,
  PixelRatio,
  StyleSheet,
  View,
  ViewStyle,
  StyleProp,
} from 'react-native';
import { BrandBackground, colors, RadialFill } from '@dcl/ui-native';
import { core } from '../core';
import { CameraFocus, Control } from '../core/native';

const REPORT_INTERVAL_MS = 50;
/** Avatar windows on screen: a screen that replaces another one with its own window takes the
 * preview over instead of closing and reopening it (the engine takes ~0.5 s to show it again). */
let mountedHoles = 0;
/** Figma `Background/Avatar` as drawn in the portrait frames (720 x 1114 from the top): noise, blend
 * modes and blur that React Native cannot render, exported as an image as its description asks. */
const spotlight = require('../assets/backpack-avatar-spotlight.webp');

/** Portrait stage: the `Background` Glow (DS `BrandBackground`) and the avatar spotlight over it. */
export function PortraitStage() {
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <BrandBackground glow />
      <Image
        source={spotlight}
        style={styles.spotlight}
        resizeMode="stretch"
        fadeDuration={0}
      />
    </View>
  );
}

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
 * Transparent window the engine draws the 3D avatar into. The rectangle is reported to the core in device
 * pixels whenever it changes (sheet drag, rotation); closes the preview when it unmounts.
 */
export function AvatarHole({
  style,
  focus,
  ready = true,
  children,
}: {
  style?: StyleProp<ViewStyle>;
  /** False while the layout around the window is still settling: nothing is reported yet. */
  ready?: boolean;
  /** Camera framing applied once the preview exists. */
  focus?: CameraFocus;
  children?: ReactNode;
}) {
  const ref = useRef<ComponentRef<typeof View>>(null);
  const opened = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const alive = useRef(true);

  const report = useCallback(() => {
    if (!ready) {
      return;
    }
    ref.current?.measureInWindow((x, y, w, h) => {
      if (!alive.current || w <= 0 || h <= 0) {
        return;
      }
      const scale = PixelRatio.get();
      const first = !opened.current;
      opened.current = true;
      core().showAvatarPreview({
        x: x * scale,
        y: y * scale,
        width: w * scale,
        height: h * scale,
      });
      if (first && focus !== undefined) {
        core().control(new Control.FocusCamera({ focus }));
      }
    });
  }, [focus, ready]);

  const schedule = useCallback(() => {
    // The first rectangle goes out at once: the preview shows as soon as the window has a size.
    if (!opened.current) {
      report();
      return;
    }
    if (timer.current) {
      return;
    }
    timer.current = setTimeout(() => {
      timer.current = null;
      report();
    }, REPORT_INTERVAL_MS);
  }, [report]);

  useEffect(() => {
    if (ready) {
      report();
    }
  }, [ready, report]);

  useEffect(() => {
    mountedHoles += 1;
    return () => {
      mountedHoles -= 1;
      alive.current = false;
      if (timer.current) {
        clearTimeout(timer.current);
      }
      const wasOpen = opened.current;
      // Decided after the commit, once a replacing screen has mounted its own window.
      requestAnimationFrame(() => {
        if (mountedHoles > 0) {
          return;
        }
        // In world the preview is an overlay the state does not show, so hide it there too.
        // Entering a world from the preview already closed it, and closing it again is an
        // invalid call.
        const tag = core().state().tag;
        if (tag === 'AvatarPreview' || (tag === 'InWorld' && wasOpen)) {
          try {
            core().hideAvatarPreview();
          } catch {
            // Already closed by entering the world.
          }
        }
      });
    };
  }, []);

  return (
    <View ref={ref} style={style} pointerEvents="none" onLayout={schedule}>
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  spotlight: {
    position: 'absolute',
    top: 0,
    left: 0,
    width: '100%',
    aspectRatio: 720 / 1114,
  },
});
