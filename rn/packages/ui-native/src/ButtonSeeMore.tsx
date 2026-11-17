import { Pressable, StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { t } from './i18n';
import { alpha, color, control } from './tokens';

/** Text toggle that expands or collapses a list or text block: "See more" while collapsed, "See
 * less" while expanded. Link colour, Text/primary pressed, White 30% disabled.
 * Figma Button/SeeMore: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=998-2402
 */
export function ButtonSeeMore({
  expanded = false,
  moreLabel = t('APP_SEE_MORE'),
  lessLabel = t('APP_SEE_LESS'),
  disabled = false,
  onPress,
}: {
  expanded?: boolean;
  moreLabel?: string;
  lessLabel?: string;
  disabled?: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ disabled, expanded }}
      disabled={disabled}
      onPress={onPress}
      style={styles.button}
    >
      {({ pressed }) => (
        <AppText
          variant="Tags"
          style={{
            color: disabled
              ? alpha['white-30']
              : pressed
              ? color.text.primary
              : color.text.link,
          }}
        >
          {expanded ? lessLabel : moreLabel}
        </AppText>
      )}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    height: control.sm,
    alignSelf: 'flex-start',
    alignItems: 'center',
    justifyContent: 'center',
  },
});
