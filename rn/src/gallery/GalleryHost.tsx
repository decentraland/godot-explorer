import { useCallback, useEffect, useRef } from 'react';
import { LayoutChangeEvent, StyleSheet, View } from 'react-native';
import { AppText, colors, u } from '../ds';
import { setOrientation } from '../GodotBridge';
import { GalleryList } from './GalleryList';
import { getPreview } from './registry';
import { PreviewRoute } from './url';

const SETTLE_MS = 800;
const ORIENTATION_WAIT_MS = 1500;

/** Renders one registered screen full-bleed and logs DCL_PREVIEW_READY for the capture script. */
export function GalleryHost({
  id,
  orientation,
  state,
}: { id: string | null } & Pick<PreviewRoute, 'orientation' | 'state'>) {
  const entry = id ? getPreview(id) : undefined;
  const wanted = orientation ?? entry?.orientation;
  const size = useRef<{ width: number; height: number } | null>(null);
  const logged = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const markReady = useCallback(() => {
    if (!id || logged.current) {
      return;
    }
    logged.current = true;
    console.log(`DCL_PREVIEW_READY ${id}`);
  }, [id]);

  useEffect(() => {
    if (wanted) {
      setOrientation(wanted);
    }
    // Without the engine nothing rotates the activity: report ready anyway, flagged.
    const fallback = setTimeout(() => {
      if (!logged.current && size.current) {
        console.log(`DCL_PREVIEW_WARN ${id} orientation not applied`);
        markReady();
      }
    }, ORIENTATION_WAIT_MS + (entry?.settleMs ?? SETTLE_MS));
    return () => {
      clearTimeout(fallback);
      clearTimeout(timer.current);
    };
  }, [id, wanted, entry?.settleMs, markReady]);

  const onLayout = (e: LayoutChangeEvent) => {
    const { width, height } = e.nativeEvent.layout;
    size.current = { width, height };
    const shape = width > height ? 'landscape' : 'portrait';
    if (wanted && shape !== wanted) {
      return;
    }
    clearTimeout(timer.current);
    timer.current = setTimeout(markReady, entry?.settleMs ?? SETTLE_MS);
  };

  if (!id) {
    return <GalleryList />;
  }
  if (!entry) {
    return (
      <View style={styles.root} testID="gallery-host">
        <AppText variant="heading" style={styles.msg}>
          Unknown preview {id}
        </AppText>
        <GalleryList />
      </View>
    );
  }
  return (
    <View style={styles.root} testID="gallery-host" onLayout={onLayout}>
      {entry.render(state)}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.purple550 },
  msg: { padding: u(16) },
});
