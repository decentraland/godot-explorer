import { StyleProp, View, ViewStyle, StyleSheet } from 'react-native';
import { colors } from './tokens';
import { u } from './scale';

/** The small red dot Figma's `Badge/Notification` (22×22) overlays on icons, pills and buttons.
 * Caller positions it (absolute top-right by convention) via `style`. `size` is in dp: 11 on icons
 * and buttons, 8 next to a `MenuNavItem` label.
 * Figma Badge/Notification: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=501-24
 */
export function NotificationBadge({
  size = 11,
  style,
}: {
  size?: number;
  style?: StyleProp<ViewStyle>;
}) {
  return (
    <View
      style={[
        styles.dot,
        { width: u(size), height: u(size), borderRadius: u(size / 2) },
        style,
      ]}
    />
  );
}

const styles = StyleSheet.create({
  dot: { backgroundColor: colors.ruby },
});
