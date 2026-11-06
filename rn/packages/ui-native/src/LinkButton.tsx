import { StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { PressableBase } from './internal/PressableBase';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** Figma `Button/Link`: an underlined `text/link` label (Inter Medium 24) for secondary actions
 * such as "Resend code" — 54px tall touch area, width hugs the label, 70% opacity while pressed. */
export function LinkButton({
  label,
  onPress,
  disabled = false,
}: {
  label: string;
  onPress: () => void;
  disabled?: boolean;
}) {
  return (
    <PressableBase
      onPress={onPress}
      disabled={disabled}
      accessibilityLabel={label}
      style={pressed => [styles.link, pressed && !disabled && styles.pressed]}
    >
      <AppText
        variant="caption"
        style={[styles.label, disabled && styles.labelDisabled]}
      >
        {label}
      </AppText>
    </PressableBase>
  );
}

const styles = StyleSheet.create({
  link: {
    height: u(27),
    alignSelf: 'flex-start',
    alignItems: 'center',
    justifyContent: 'center',
  },
  pressed: { opacity: 0.7 },
  label: {
    fontFamily: fonts.medium,
    fontSize: u(12),
    color: colors.link,
    textDecorationLine: 'underline',
  },
  labelDisabled: { color: colors.textDisabled },
});
