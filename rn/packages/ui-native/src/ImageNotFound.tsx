import { StyleProp, StyleSheet, View, ViewStyle } from 'react-native';
import { Icon } from './Icon';
import { colors, radius } from './tokens';
import { u } from './scale';

/** Figma `Image Not Found`: the deep-purple box with the crossed-image glyph that stands in for a
 * missing or failed image. Fills its parent unless `style` sizes it; `iconSize` 36 is the
 * 294×156 Figma frame (the glyph is 24.5% of the width).
 * Figma Image Not Found: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=821-3478
 */
export function ImageNotFound({
  iconSize = 36,
  style,
}: {
  iconSize?: number;
  style?: StyleProp<ViewStyle>;
}) {
  return (
    <View style={[styles.box, style]}>
      <Icon name="imageOff" size={iconSize} color={colors.lavender70} />
    </View>
  );
}

const styles = StyleSheet.create({
  box: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: radius.sm,
    borderWidth: u(0.3),
    borderColor: colors.lavender50,
    backgroundColor: colors.deepPurple50,
  },
});
