import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { t } from './i18n';
import { color, colors, minTouchTarget, radius, space } from './tokens';
import { u } from './scale';

/** Chip link for a profile link (website, socials): icon and label in Text/link on Surface/overlay.
 * `onRemove` shows the close badge of the edit mode.
 * Figma Button/ProfileLink: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1003-232
 */
export function ButtonProfileLink({
  label,
  icon = 'link',
  onPress,
  onRemove,
}: {
  label: string;
  icon?: IconName;
  onPress: () => void;
  onRemove?: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="link"
      accessibilityLabel={label}
      onPress={onPress}
      style={({ pressed }) => [
        styles.chip,
        {
          backgroundColor: pressed
            ? color.surface['overlay-pressed']
            : color.surface.overlay,
        },
      ]}
    >
      <Icon name={icon} size={12} color={color.text.link} />
      <AppText variant="B4" style={styles.label}>
        {label}
      </AppText>
      {onRemove && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('APP_REMOVE')}
          hitSlop={(minTouchTarget - u(10)) / 2}
          onPress={onRemove}
          style={styles.remove}
        >
          <View>
            <Icon name="close" size={7} color={color.icon.default} />
          </View>
        </Pressable>
      )}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  chip: {
    height: u(32),
    flexDirection: 'row',
    alignItems: 'center',
    alignSelf: 'flex-start',
    gap: space['2xs'],
    paddingHorizontal: space['10'],
    paddingVertical: space['2xs'],
    borderRadius: radius.xl,
  },
  label: { color: color.text.link },
  remove: {
    height: u(10),
    paddingHorizontal: u(2),
    justifyContent: 'center',
    borderRadius: u(3),
    backgroundColor: colors.black,
  },
});
