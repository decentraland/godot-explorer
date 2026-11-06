import { ReactNode } from 'react';
import { Pressable, StyleProp, ViewStyle } from 'react-native';

/** Shared Pressable mechanics — accessibility wiring and disabled gating — for every
 * button-family component in `ds/` (Button, PillButton, IconButton, LoginButton). The visual
 * response to press/disabled stays with each component: `style` gets the live `pressed` flag so a
 * component with real per-state Figma colors (IconButton, PillButton) can switch to those instead
 * of a flat opacity dim, which Button still does by folding it into its own `style` callback. */
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
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel}
      accessibilityState={{ disabled, ...accessibilityState }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => style(pressed)}
    >
      {typeof children === 'function'
        ? ({ pressed }) => children(pressed)
        : children}
    </Pressable>
  );
}
