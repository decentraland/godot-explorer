import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useMemo,
  useRef,
} from 'react';
import {
  Animated,
  GestureResponderHandlers,
  PanResponder,
  Pressable,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { colors, elevation, radii } from './tokens';
import {
  DEFAULT_SNAP_CONFIG,
  nearestSnap,
  Snap,
  SnapConfig,
  snapHeights,
} from './sheetMath';
import { u } from './scale';

const ORDER: Snap[] = ['peek', 'half', 'full'];

const SheetDrag = createContext<GestureResponderHandlers>({});

/** The handlers that drag the sheet by a vertical move, for a strip inside it (e.g. the tab row)
 * that should drag as well. They never claim a tap, so its buttons keep working. */
export function useSheetDrag(): GestureResponderHandlers {
  return useContext(SheetDrag);
}

/**
 * Sheet that rests at peek / half / full. Only the handle + `header` area
 * drags, so scrollable `children` keep their own gestures. `snap` is
 * controlled: the sheet reports the target through `onSnapChange`.
 */
export function BottomSheet({
  snap,
  onSnapChange,
  containerHeight,
  config = DEFAULT_SNAP_CONFIG,
  header,
  background,
  handleWidth,
  handlePadding,
  backgroundHeight,
  onMovingChange,
  onVisibleChange,
  children,
}: {
  snap: Snap;
  onSnapChange: (snap: Snap) => void;
  containerHeight?: number;
  config?: SnapConfig;
  header?: ReactNode;
  /** Drawn behind the handle and body (e.g. a gradient); the solid fill stays underneath. */
  background?: ReactNode;
  /** Handle width in dp; defaults to 40. */
  handleWidth?: number;
  /** Vertical padding around the handle in dp; defaults to 10. */
  handlePadding?: number;
  /** dp the `background` covers from the sheet's top; over the visible part when that is shorter,
   * so its last colour meets what sits under the sheet at every snap. */
  backgroundHeight?: number;
  /** True from the moment a drag starts until the sheet has come to rest. */
  onMovingChange?: (moving: boolean) => void;
  /** dp of the sheet above the container's bottom, on every frame it moves and once at rest. */
  onVisibleChange?: (visible: number) => void;
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
  const current = useRef(heights.full - heights[snap]);
  const latest = useRef({
    heights,
    onSnapChange,
    onMovingChange,
    onVisibleChange,
    snap,
  });
  latest.current = {
    heights,
    onSnapChange,
    onMovingChange,
    onVisibleChange,
    snap,
  };

  // The value is read back synchronously: a drag that starts mid-spring must begin from where
  // the sheet is, not from where it was a frame ago.
  useEffect(() => {
    const id = offset.addListener(({ value }) => {
      current.current = value;
      latest.current.onVisibleChange?.(latest.current.heights.full - value);
    });
    return () => offset.removeListener(id);
  }, [offset]);

  useEffect(
    () => latest.current.onVisibleChange?.(heights.full - current.current),
    [heights],
  );

  const settle = useMemo(
    () => (target: number) => {
      const toValue = latest.current.heights.full - target;
      if (Math.abs(current.current - toValue) < 0.5) {
        latest.current.onMovingChange?.(false);
        return;
      }
      latest.current.onMovingChange?.(true);
      Animated.spring(offset, {
        toValue,
        useNativeDriver: false,
        bounciness: 0,
      }).start(({ finished }) => {
        if (finished) {
          latest.current.onMovingChange?.(false);
        }
      });
    },
    [offset],
  );

  useEffect(() => settle(heights[snap]), [snap, heights, settle]);

  const [grab, area] = useMemo(() => {
    const visibleAfter = (dy: number) => {
      const { peek, full } = latest.current.heights;
      return Math.min(full, Math.max(peek, dragStart.current - dy));
    };
    const release = (vy: number, dy: number) => {
      const { heights: h, onSnapChange: change } = latest.current;
      const next = nearestSnap(visibleAfter(dy), vy, h);
      settle(h[next]);
      change(next);
    };
    const toggle = () => {
      const { snap: now, onSnapChange: change } = latest.current;
      change(now === 'full' ? 'half' : ORDER[ORDER.indexOf(now) + 1]);
    };
    // `grab` takes the touch the moment it lands (the handle, which has no buttons of its own to
    // protect); `area` waits for a vertical move, so what is inside it still gets its taps.
    const make = (onTouch: boolean) =>
      PanResponder.create({
        onStartShouldSetPanResponderCapture: () => onTouch,
        onMoveShouldSetPanResponder: (_, g) =>
          Math.abs(g.dy) > 4 && Math.abs(g.dy) > Math.abs(g.dx),
        // Once the sheet is being dragged nothing else (a scroll view) takes it.
        onPanResponderTerminationRequest: () => false,
        onPanResponderGrant: () => {
          offset.stopAnimation();
          dragStart.current = latest.current.heights.full - current.current;
          latest.current.onMovingChange?.(true);
        },
        onPanResponderMove: (_, g) => {
          offset.setValue(latest.current.heights.full - visibleAfter(g.dy));
        },
        onPanResponderRelease: (_, g) => {
          if (onTouch && Math.abs(g.dx) < 6 && Math.abs(g.dy) < 6) {
            toggle();
            return;
          }
          release(g.vy, g.dy);
        },
        onPanResponderTerminate: (_, g) => release(0, g.dy),
      });
    return [make(true).panHandlers, make(false).panHandlers];
  }, [offset, settle]);

  const fitted = useMemo(() => {
    if (!backgroundHeight) {
      return undefined;
    }
    const { peek, full } = heights;
    const shrinks = backgroundHeight > peek && full > backgroundHeight;
    const input = [0, Math.max(0.001, full - backgroundHeight), full - peek];
    // The layer shrinks from its top edge once the visible part is shorter than it.
    const scaleY = shrinks
      ? offset.interpolate({
          inputRange: input,
          outputRange: [1, 1, peek / backgroundHeight],
          extrapolate: 'clamp',
        })
      : 1;
    const translateY = shrinks
      ? offset.interpolate({
          inputRange: input,
          outputRange: [0, 0, -(backgroundHeight - peek) / 2],
          extrapolate: 'clamp',
        })
      : 0;
    return {
      height: backgroundHeight,
      transform: [{ translateY }, { scaleY }],
    };
  }, [backgroundHeight, offset, heights]);

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
      {fitted ? (
        <Animated.View pointerEvents="none" style={[styles.backdrop, fitted]}>
          {background}
        </Animated.View>
      ) : (
        background
      )}
      <View {...grab}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={expanded ? 'Collapse sheet' : 'Expand sheet'}
          hitSlop={12}
          onPress={() => onSnapChange(target)}
          style={[
            styles.handleArea,
            handlePadding !== undefined && {
              paddingVertical: u(handlePadding),
            },
          ]}
        >
          <View
            style={[
              styles.handle,
              handleWidth ? { width: u(handleWidth) } : null,
            ]}
          />
        </Pressable>
        {header}
      </View>
      <SheetDrag.Provider value={area}>
        <View style={styles.body}>{children}</View>
      </SheetDrag.Provider>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  sheet: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    borderTopLeftRadius: radii.sheet,
    borderTopRightRadius: radii.sheet,
    backgroundColor: colors.purple500,
    overflow: 'hidden',
    ...elevation.up,
  },
  backdrop: { position: 'absolute', top: 0, left: 0, right: 0 },
  handleArea: { alignItems: 'center', paddingVertical: u(10) },
  handle: {
    width: u(40),
    height: u(4),
    borderRadius: u(2),
    backgroundColor: colors.white30,
  },
  body: { flex: 1 },
});
