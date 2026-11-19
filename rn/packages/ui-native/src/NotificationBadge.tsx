import { StyleProp, View, ViewStyle, StyleSheet } from 'react-native';
import { colors } from './tokens';
import { u } from './scale';

/** The small red dot Figma's `Badge/Notification` (22×22) overlays on icons, pills and buttons.
 * Caller positions it (absolute top-right by convention) via `style`.
 * Figma Badge/Notification: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=501-24
 */
export function NotificationBadge({ style }: { style?: StyleProp<ViewStyle> }) {
  return <View style={[styles.dot, style]} />;
}

const styles = StyleSheet.create({
  dot: {
    width: u(11),
    height: u(11),
    borderRadius: u(5.5),
    backgroundColor: colors.ruby,
  },
});
