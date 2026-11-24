import { Image, Pressable, StyleSheet, View } from 'react-native';
import { Icon } from './Icon';
import { t } from './i18n';
import { color, colors, elevation, radius } from './tokens';
import { u } from './scale';

/** Screenshot attachment tile: the image with a remove button, or empty with a + to add one.
 * Pressed shows the lavender border.
 * Figma Card/Screenshot: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1098-496
 */
export function ScreenshotCard({
  uri,
  onPress,
  onRemove,
  accessibilityLabel,
}: {
  /** Without it the tile is Figma's `Empty=True`. */
  uri?: string;
  onPress: () => void;
  onRemove?: () => void;
  accessibilityLabel: string;
}) {
  const empty = uri === undefined;
  return (
    <View style={styles.wrap}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={accessibilityLabel}
        onPress={onPress}
        style={({ pressed }) => [
          styles.tile,
          empty ? styles.empty : styles.filled,
          pressed && styles.pressed,
        ]}
      >
        {empty ? (
          <View style={styles.add}>
            <Icon name="add" size={27} color={color.icon.default} />
          </View>
        ) : (
          <Image source={{ uri }} style={StyleSheet.absoluteFill} />
        )}
      </Pressable>
      {!empty && onRemove && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('APP_REMOVE')}
          onPress={onRemove}
          hitSlop={u(14)}
          style={styles.remove}
        >
          <Icon name="close" size={11} color={color.icon.default} />
        </Pressable>
      )}
    </View>
  );
}

const SIZE = u(85);

const styles = StyleSheet.create({
  wrap: { width: SIZE, height: SIZE, ...elevation.down },
  tile: {
    width: SIZE,
    height: SIZE,
    borderRadius: radius.lg,
    borderWidth: 0.5,
    overflow: 'hidden',
    alignItems: 'center',
    justifyContent: 'center',
  },
  filled: { borderColor: colors.pearl },
  empty: { backgroundColor: color.surface.overlay, borderColor: colors.shadow },
  pressed: { borderWidth: 1, borderColor: colors.lavender },
  add: { opacity: 0.3 },
  remove: {
    position: 'absolute',
    top: -u(4.5),
    right: -u(4),
    width: u(16),
    height: u(16),
    borderRadius: radius.sm,
    backgroundColor: colors.shadow,
    alignItems: 'center',
    justifyContent: 'center',
  },
});
