import { ReactNode, useEffect, useMemo, useRef } from 'react';
import {
  Animated,
  Easing,
  PanResponder,
  Pressable,
  StyleSheet,
  View,
} from 'react-native';
import { colors, LinearFill, radii } from '../ds';
import { SheetState } from './backpackState';
import { u } from '../ds/scale';

const HANDLE_AREA = u(28);
const DRAG_THRESHOLD = 24;

/** Bottom sheet that snaps between a full height and just its handle (Figma "Full avatar"). */
export function Sheet({
  height,
  state,
  onState,
  children,
}: {
  height: number;
  state: SheetState;
  onState: (next: SheetState) => void;
  children: ReactNode;
}) {
  const animated = useRef(
    new Animated.Value(state === 'expanded' ? height : HANDLE_AREA),
  ).current;
  const latest = useRef({ state, onState });
  latest.current = { state, onState };

  useEffect(() => {
    Animated.timing(animated, {
      toValue: state === 'expanded' ? height : HANDLE_AREA,
      duration: 220,
      easing: Easing.out(Easing.cubic),
      useNativeDriver: false,
    }).start();
  }, [animated, height, state]);

  const responder = useMemo(
    () =>
      PanResponder.create({
        onStartShouldSetPanResponder: () => true,
        onPanResponderRelease: (_event, gesture) => {
          const { state: current, onState: change } = latest.current;
          if (gesture.dy < -DRAG_THRESHOLD) {
            change('expanded');
          } else if (gesture.dy > DRAG_THRESHOLD) {
            change('collapsed');
          } else if (Math.abs(gesture.dy) < 6) {
            change(current === 'expanded' ? 'collapsed' : 'expanded');
          }
        },
      }),
    [],
  );

  return (
    <Animated.View style={[styles.sheet, { height: animated }]}>
      <LinearFill
        stops={[
          { offset: 0, color: colors.purple450 },
          { offset: 1, color: colors.purple550 },
        ]}
      />
      <Pressable
        accessibilityRole="adjustable"
        accessibilityLabel="Resize panel"
        style={styles.handleArea}
        {...responder.panHandlers}
      >
        <View style={styles.handle} />
      </Pressable>
      <View style={styles.body}>{children}</View>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  sheet: {
    overflow: 'hidden',
    borderTopLeftRadius: radii.sheet * 2,
    borderTopRightRadius: radii.sheet * 2,
  },
  handleArea: {
    height: HANDLE_AREA,
    alignItems: 'center',
    justifyContent: 'center',
  },
  handle: {
    width: u(100),
    height: u(4),
    borderRadius: u(2),
    backgroundColor: 'rgba(252,252,252,0.3)',
  },
  body: { flex: 1 },
});
