import { StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { PressableBase } from './internal/PressableBase';
import { Spinner } from './Spinner';
import {
  border,
  colors,
  controlHeight,
  fonts,
  iconSize,
  radius,
  space,
} from './tokens';
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
   * Height, radius, label and icon size all change per size (96/16, 74/12 or 10, 54/8). */
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
  const glyph = big
    ? iconSize.buttonLg
    : small
    ? iconSize.buttonSm
    : iconSize.buttonMd;
  const medium = !big && !small && !reminder;
  return (
    <PressableBase
      onPress={onPress}
      disabled={inactive}
      accessibilityLabel={label}
      accessibilityState={{ disabled: inactive, ...(reminder && { selected }) }}
      style={pressed => [
        styles.button,
        (big || reminder) && styles.big,
        medium && styles.medium,
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
        <Spinner size={big ? 16 : small ? 12 : 14} />
      ) : (
        <>
          {icon && (
            <Icon name={icon} size={reminder ? 27 : glyph} color={labelColor} />
          )}
          <AppText
            variant="button"
            style={[styles.label, { color: labelColor, fontSize }]}
          >
            {label}
          </AppText>
          {trailingIcon && (
            <Icon name={trailingIcon} size={glyph} color={labelColor} />
          )}
        </>
      )}
    </PressableBase>
  );
}

const styles = StyleSheet.create({
  button: {
    minHeight: controlHeight.md,
    paddingHorizontal: space.lg,
    borderRadius: radius.xl,
    gap: space.xs,
    alignItems: 'center',
    justifyContent: 'center',
  },
  big: { minHeight: controlHeight.lg },
  medium: { borderRadius: radius.lg },
  small: { minHeight: controlHeight.sm, borderRadius: radius.sm },
  label: { fontFamily: fonts.bold, textTransform: 'uppercase' },
  fullWidth: { alignSelf: 'stretch' },
  primary: { backgroundColor: colors.ruby },
  secondary: { backgroundColor: colors.black40 },
  outline: {
    backgroundColor: colors.black40,
    borderWidth: border.thin,
    borderColor: colors.snow,
  },
  dark: { backgroundColor: colors.black40 },
  reminder: {
    backgroundColor: colors.glass,
    borderRadius: radius.lg,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: colors.snow,
  },
  reminderSelected: {
    backgroundColor: colors.black20,
    borderWidth: border.thin,
    borderColor: colors.ruby,
  },
  reminderPressed: { backgroundColor: colors.glassPressed },
  withIcon: { flexDirection: 'row' },
  primaryDisabled: { backgroundColor: colors.rubyDisabled },
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
