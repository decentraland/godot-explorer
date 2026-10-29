import { ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { LayoutChangeEvent, LogBox, StyleSheet, View } from 'react-native';
import { signedInMockCoreForPreviews } from '../core';
import { colors } from '../ds';
import { Orientation, setOrientation } from '../platform/orientation';

// The dev warning toast would sit in the corner of every capture.
LogBox.ignoreAllLogs();
signedInMockCoreForPreviews();

const SETTLE_MS = 800;
const ORIENTATION_WAIT_MS = 1500;

/**
 * Renders one screen full-bleed in its own orientation. Once laid out and settled it logs
 * DCL_PREVIEW_READY (the adb capture script) and mounts the `preview-ready` element (Maestro).
 */
export function PreviewFrame({
  id,
  orientation,
  settleMs = SETTLE_MS,
  children,
}: {
  id: string;
  orientation?: Orientation;
  settleMs?: number;
  children: ReactNode;
}) {
  const size = useRef<{ width: number; height: number } | null>(null);
  const logged = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [ready, setReady] = useState(false);

  const markReady = useCallback(() => {
    if (logged.current) {
      return;
    }
    logged.current = true;
    console.log(`DCL_PREVIEW_READY ${id}`);
    setReady(true);
  }, [id]);

  useEffect(() => {
    if (orientation) {
      setOrientation(orientation);
    }
    // Without the engine nothing rotates the activity: report ready anyway, flagged.
    const fallback = setTimeout(() => {
      if (!logged.current && size.current) {
        console.log(`DCL_PREVIEW_WARN ${id} orientation not applied`);
        markReady();
      }
    }, ORIENTATION_WAIT_MS + settleMs);
    return () => {
      clearTimeout(fallback);
      clearTimeout(timer.current);
    };
  }, [id, orientation, settleMs, markReady]);

  const onLayout = (e: LayoutChangeEvent) => {
    const { width, height } = e.nativeEvent.layout;
    size.current = { width, height };
    const shape = width > height ? 'landscape' : 'portrait';
    if (orientation && shape !== orientation) {
      return;
    }
    clearTimeout(timer.current);
    timer.current = setTimeout(markReady, settleMs);
  };

  return (
    <View style={styles.root} testID="gallery-host" onLayout={onLayout}>
      {children}
      {ready ? (
        <View accessible testID="preview-ready" style={styles.flag} />
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.purple550 },
  // Transparent: it is in the view hierarchy for the test runner, not in the capture.
  flag: { position: 'absolute', top: 0, left: 0, width: 2, height: 2 },
});
