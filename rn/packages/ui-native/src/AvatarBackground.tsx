import { Image, StyleProp, StyleSheet, ImageStyle } from 'react-native';

const spotlight = require('../assets/avatar-background.png');

/** Figma `Background/Avatar`: the purple spotlight behind the 3D avatar (create and naming
 * screens). Its noise, blend modes and layer blur have no React Native equivalent, so it ships
 * as an image; it fills its parent unless `style` sizes it.
 * Figma Background/Avatar: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=677-2
 */
export function AvatarBackground({ style }: { style?: StyleProp<ImageStyle> }) {
  return (
    <Image
      source={spotlight}
      resizeMode="contain"
      style={[styles.fill, style]}
    />
  );
}

const styles = StyleSheet.create({
  fill: { ...StyleSheet.absoluteFill, width: '100%', height: '100%' },
});
