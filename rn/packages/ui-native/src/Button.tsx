import { ActivityIndicator, StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { PressableBase } from './internal/PressableBase';
import { colors, radii } from './tokens';
import { u } from './scale';

export type ButtonTone =
  | 'primary'
  | 'secondary'
  | 'ghost'
  | 'dark'
  | 'outline'
  | 'reminder';

export function Button({
  label,
  onPress,
  tone = 'primary',
  disabled = false,
  loading = false,
  selected = false,
  big = false,
  small = false,
  fullWidth = false,
  icon,
  trailingIcon,
}: {
  label: string;
  onPress: () => void;
  /** `reminder` is Figma's `Button/Reminder` toggle (bell + "Remind me"): purple overlay by
   * default, Ruby text and border on black when `selected`. The caller swaps the icon. */
  tone?: ButtonTone;
  disabled?: boolean;
  loading?: boolean;
  /** `tone="reminder"` only — the reminder is set. */
  selected?: boolean;
  /** Figma `Button/Primary|Secondary` `Size=Large` — hero CTAs (entry-flow, Settings actions).
   * Only height and label size change between sizes, not the corner radius. */
  big?: boolean;
  /** `Size=Small` — compact inline actions. The default (neither flag) is `Size=Medium`. */
  small?: boolean;
  /** Figma's `Fill container` width mode (default is `Hug`). */
  fullWidth?: boolean;
  /** Leading icon, drawn in the label colour (Figma's `leadingIcon`). */
  icon?: IconName;
  /** Trailing icon (Figma's `trailingIcon`, e.g. `jumpIn` for `Type=JumpIn`). */
  trailingIcon?: IconName;
}) {
  const inactive = disabled || loading;
  /** `ghost` has no Figma source (see §Disabled below) — everything else gets the explicit
   * `theme.colors.text.disabled` swatch instead of a dimmed tone colour. */
  const reminder = tone === 'reminder';
  const labelColor =
    tone === 'ghost'
      ? colors.gray
      : reminder && selected
      ? colors.ruby
      : inactive
      ? colors.textDisabled
      : colors.snow;
  const fontSize = big || reminder ? u(15) : small ? u(11) : u(12);
  return (
    <PressableBase
      onPress={onPress}
      disabled={inactive}
      accessibilityLabel={label}
      accessibilityState={{ disabled: inactive, ...(reminder && { selected }) }}
      style={pressed => [
        styles.button,
        (big || reminder) && styles.big,
        small && styles.small,
        fullWidth && styles.fullWidth,
        tone === 'primary' && styles.primary,
        tone === 'secondary' && styles.secondary,
        tone === 'dark' && styles.dark,
        tone === 'outline' && styles.outline,
        (icon || trailingIcon) && styles.withIcon,
        reminder && styles.reminder,
        reminder && selected && styles.reminderSelected,
        // Disabled: Figma gives each tone its own explicit bg/border (`theme.colors.*.disabled`),
        // not a flat dim — a generic opacity wrapper double-darkens tones whose base colour is
        // already translucent (secondary/dark/outline all start at black @ 0.4-0.6).
        inactive && tone === 'primary' && styles.primaryDisabled,
        inactive &&
          (tone === 'secondary' || tone === 'dark') &&
          styles.secondaryDisabled,
        inactive && tone === 'outline' && styles.outlineDisabled,
        // `ghost` and `reminder` have no Figma-sourced disabled swatch — the generic dim is the
        // honest fallback.
        inactive && (tone === 'ghost' || reminder) && styles.genericDisabled,
        // Pressed: real explicit swatches, not an opacity dim — primary's #E1002B isn't reachable
        // by dimming #FF2D55 against an arbitrary background.
        pressed && !inactive && tone === 'primary' && styles.primaryPressed,
        pressed &&
          !inactive &&
          (tone === 'secondary' || tone === 'dark' || tone === 'outline') &&
          styles.darkPressed,
        pressed && !inactive && tone === 'ghost' && styles.genericPressed,
        pressed && !inactive && reminder && !selected && styles.reminderPressed,
      ]}
    >
      {loading ? (
        <ActivityIndicator color={colors.snow} />
      ) : (
        <>
          {icon && (
            <Icon
              name={icon}
              size={reminder ? 27 : big ? 20 : 16}
              color={labelColor}
            />
          )}
          <AppText variant="button" style={{ color: labelColor, fontSize }}>
            {label}
          </AppText>
          {trailingIcon && (
            <Icon name={trailingIcon} size={14} color={labelColor} />
          )}
        </>
      )}
    </PressableBase>
  );
}

const styles = StyleSheet.create({
  button: {
    minHeight: u(37),
    paddingHorizontal: u(12),
    borderRadius: radii.button,
    alignItems: 'center',
    justifyContent: 'center',
  },
  big: { minHeight: u(46) },
  small: { minHeight: u(27) },
  fullWidth: { alignSelf: 'stretch' },
  primary: { backgroundColor: colors.ruby },
  secondary: { backgroundColor: colors.black40 },
  outline: {
    backgroundColor: colors.black40,
    borderWidth: 2,
    borderColor: colors.snow,
  },
  dark: { backgroundColor: colors.black40 },
  reminder: {
    backgroundColor: colors.glass,
    borderRadius: u(6),
    borderWidth: 2,
    borderColor: 'transparent',
    gap: u(4),
  },
  reminderSelected: {
    backgroundColor: colors.black20,
    borderColor: colors.ruby,
  },
  reminderPressed: { backgroundColor: colors.glassPressed },
  withIcon: { flexDirection: 'row', gap: u(8) },
  primaryDisabled: { backgroundColor: 'rgba(255,45,85,0.5)' },
  secondaryDisabled: { backgroundColor: colors.black30 },
  outlineDisabled: {
    backgroundColor: colors.black30,
    borderColor: colors.textDisabled,
  },
  genericDisabled: { opacity: 0.5 },
  primaryPressed: { backgroundColor: colors.rubyPressed },
  darkPressed: { backgroundColor: colors.black60 },
  genericPressed: { opacity: 0.85 },
});
