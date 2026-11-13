import { ReactNode, useEffect, useMemo, useRef } from 'react';
import {
  Animated,
  PanResponder,
  Pressable,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { SheetGlow } from './Sheet';
import { colors, elevation, radius } from './tokens';
import {
  DEFAULT_SNAP_CONFIG,
  nearestSnap,
  Snap,
  SnapConfig,
  snapHeights,
} from './sheetMath';
import { u } from './scale';

const ORDER: Snap[] = ['peek', 'half', 'full'];

/** Space the handle takes above the content: Figma `Sheet` pt 16 px + the 8 px handle + gap 40 px. */
export const SHEET_HANDLE_HEIGHT = u(32);

/**
 * Figma `Sheet` (Handle=On) that rests at peek / half / full. Only the handle + `header` area
 * drags, so scrollable `children` keep their own gestures. `snap` is
 * controlled: the sheet reports the target through `onSnapChange`.
 */
export function BottomSheet({
  snap,
  onSnapChange,
  containerHeight,
  config = DEFAULT_SNAP_CONFIG,
  header,
  children,
}: {
  snap: Snap;
  onSnapChange: (snap: Snap) => void;
  containerHeight?: number;
  config?: SnapConfig;
  header?: ReactNode;
  children?: ReactNode;
}) {
  const window = useWindowDimensions();
  const height = containerHeight ?? window.height;
  const heights = useMemo(() => snapHeights(height, config), [height, config]);
  // translateY = how far the sheet is pushed below its full-height position.
  const offset = useRef(
    new Animated.Value(heights.full - heights[snap]),
  ).current;
  const dragStart = useRef(0);
  const latest = useRef({ heights, onSnapChange, snap });
  latest.current = { heights, onSnapChange, snap };

  const settle = useMemo(
    () => (target: number) =>
      Animated.spring(offset, {
        toValue: latest.current.heights.full - target,
        useNativeDriver: true,
        bounciness: 0,
      }).start(),
    [offset],
  );

  useEffect(() => settle(heights[snap]), [snap, heights, settle]);

  const responder = useMemo(
    () =>
      PanResponder.create({
        onMoveShouldSetPanResponder: (_, g) => Math.abs(g.dy) > 4,
        onPanResponderGrant: () => {
          offset.stopAnimation(value => {
            dragStart.current = latest.current.heights.full - value;
          });
        },
        onPanResponderMove: (_, g) => {
          const { peek, full } = latest.current.heights;
          const visible = Math.min(
            full,
            Math.max(peek, dragStart.current - g.dy),
          );
          offset.setValue(full - visible);
        },
        onPanResponderRelease: (_, g) => {
          const { heights: h, onSnapChange: change } = latest.current;
          const visible = Math.min(
            h.full,
            Math.max(h.peek, dragStart.current - g.dy),
          );
          const next = nearestSnap(visible, g.vy, h);
          settle(h[next]);
          change(next);
        },
      }),
    [offset, settle],
  );

  const index = ORDER.indexOf(snap);
  const expanded = snap === 'full';
  const target = expanded ? 'half' : ORDER[index + 1];

  return (
    <Animated.View
      style={[
        styles.sheet,
        { height: heights.full, transform: [{ translateY: offset }] },
      ]}
    >
      <View style={styles.clip}>
        <SheetGlow />
      </View>
      <View {...responder.panHandlers}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={expanded ? 'Collapse sheet' : 'Expand sheet'}
          hitSlop={12}
          onPress={() => onSnapChange(target)}
          style={styles.handleArea}
        >
          <View style={styles.handle} />
        </Pressable>
        {header}
      </View>
      <View style={styles.body}>{children}</View>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  sheet: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    borderTopLeftRadius: radius['2xl'],
    borderTopRightRadius: radius['2xl'],
    backgroundColor: colors.purple500,
    ...elevation.up,
  },
  clip: {
    ...StyleSheet.absoluteFill,
    borderTopLeftRadius: radius['2xl'],
    borderTopRightRadius: radius['2xl'],
    overflow: 'hidden',
  },
  handleArea: {
    alignItems: 'center',
    paddingTop: u(8),
    paddingBottom: u(20),
  },
  handle: {
    width: u(100),
    height: u(4),
    borderRadius: radius.xs,
    backgroundColor: colors.white30,
  },
  body: { flex: 1 },
});
