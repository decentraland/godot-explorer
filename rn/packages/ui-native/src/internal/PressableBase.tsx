import { ReactNode, useState } from 'react';
import { Insets, Pressable, StyleProp, ViewStyle } from 'react-native';
import { minTouchTarget } from '../tokens';

/** Shared Pressable mechanics — accessibility wiring and disabled gating — for every
 * button-family component in `ds/` (Button, PillButton, IconButton, LoginButton). The visual
 * response to press/disabled stays with each component: `style` gets the live `pressed` flag so a
 * component with real per-state Figma colors (IconButton, PillButton) can switch to those instead
 * of a flat opacity dim, which Button still does by folding it into its own `style` callback.
 * A control drawn smaller than `minTouchTarget` keeps its drawn size and gets the rest as `hitSlop`. */
export function PressableBase({
  onPress,
  disabled = false,
  accessibilityLabel,
  accessibilityState,
  style,
  children,
}: {
  onPress: () => void;
  disabled?: boolean;
  accessibilityLabel: string;
  accessibilityState?: Record<string, boolean>;
  style: (pressed: boolean) => StyleProp<ViewStyle>;
  /** A function gets the live `pressed` flag, for icons that change colour while pressed. */
  children: ReactNode | ((pressed: boolean) => ReactNode);
}) {
  const [hitSlop, setHitSlop] = useState<Insets>();
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel}
      accessibilityState={{ disabled, ...accessibilityState }}
      disabled={disabled}
      onPress={onPress}
      hitSlop={hitSlop}
      onLayout={event => {
        const { width, height } = event.nativeEvent.layout;
        const x = Math.max(0, (minTouchTarget - width) / 2);
        const y = Math.max(0, (minTouchTarget - height) / 2);
        if (hitSlop?.left !== x || hitSlop?.top !== y) {
          setHitSlop({ left: x, right: x, top: y, bottom: y });
        }
      }}
      style={({ pressed }) => style(pressed)}
    >
      {typeof children === 'function'
        ? ({ pressed }) => children(pressed)
        : children}
    </Pressable>
  );
}
