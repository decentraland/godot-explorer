import { StyleSheet } from 'react-native';
import { Icon, IconName } from './Icon';
import { PressableBase } from './internal/PressableBase';
import { border, colors, iconSize } from './tokens';
import { u } from './scale';

/** Figma's icon-only button family — `variant="filled"` covers both `Button/Discover` (`big`,
 * 94×96, purple overlay) and `Button/Filter` (default, 54×54, black overlay); `variant="outline"`
 * is `Button/Social` (54×54, 2px border, used for accept/cancel-style circular actions);
 * `variant="primary"` is `Button/Primary` `Text=False` (the 54×54 Ruby JumpIn square; `big` is the
 * 60×60 Dice square with a 54 glyph). Colors
 * are passed directly, not a `tone` enum — this family's palette varies per concrete use (purple
 * nav overlay vs. black filter overlay vs. a semantic accept/cancel border) in a way a shared tone
 * system can't express without forcing unrelated cases through the same few names. */
export function IconButton({
  icon,
  onPress,
  variant = 'filled',
  big = false,
  selected = false,
  disabled = false,
  /** `variant="filled"` Filter only: `Background=Off` draws the glyph alone, with the same per-state
   * colours. */
  background = true,
  /** `variant="outline"` only: the border and icon colour while pressed, for a semantic
   * accept/cancel flash (e.g. `colors.ruby` for cancel, a green rarity colour for accept).
   * Defaults to no change — only the background darkens on press. */
  accentColor,
  accessibilityLabel,
}: {
  icon: IconName;
  onPress: () => void;
  variant?: 'filled' | 'outline' | 'primary';
  /** `variant="filled"`: the Large Discover size (94×96) instead of the Filter size (54×54);
   * `variant="primary"`: the 60×60 Medium square instead of the 54×54 Small one. */
  big?: boolean;
  selected?: boolean;
  disabled?: boolean;
  background?: boolean;
  accentColor?: string;
  accessibilityLabel: string;
}) {
  if (variant === 'outline') {
    return (
      <PressableBase
        onPress={onPress}
        disabled={disabled}
        accessibilityLabel={accessibilityLabel}
        accessibilityState={{ disabled, selected }}
        style={pressed => [
          styles.outline,
          {
            backgroundColor: disabled
              ? colors.black30
              : pressed
              ? colors.black60
              : colors.black40,
            borderColor: disabled
              ? colors.textDisabled
              : pressed
              ? accentColor ?? colors.snow
              : colors.snow,
          },
        ]}
      >
        {pressed => (
          <Icon
            name={icon}
            size={iconSize.iconButton}
            color={(pressed && accentColor) || colors.snow}
          />
        )}
      </PressableBase>
    );
  }
  if (variant === 'primary') {
    return (
      <PressableBase
        onPress={onPress}
        disabled={disabled}
        accessibilityLabel={accessibilityLabel}
        accessibilityState={{ disabled }}
        style={pressed => [
          big ? styles.primaryBig : styles.primary,
          {
            backgroundColor: disabled
              ? colors.rubyDisabled
              : pressed
              ? colors.rubyPressed
              : colors.ruby,
          },
        ]}
      >
        <Icon
          name={icon}
          size={big ? 27 : iconSize.buttonSm}
          color={
            disabled ? (big ? colors.snow10 : colors.textDisabled) : colors.snow
          }
        />
      </PressableBase>
    );
  }
  // Filter has one `Tap-Selected` state for both the press and the selection.
  const iconColor = (pressed: boolean) =>
    disabled
      ? big
        ? colors.white30
        : colors.snow10
      : selected || pressed
      ? colors.lavender
      : colors.snow;
  return (
    <PressableBase
      onPress={onPress}
      disabled={disabled}
      accessibilityLabel={accessibilityLabel}
      accessibilityState={{ disabled, selected }}
      style={pressed =>
        big
          ? [
              styles.big,
              {
                backgroundColor: disabled
                  ? colors.glass20
                  : selected
                  ? 'rgba(105,31,169,0.7)'
                  : pressed
                  ? colors.glassPressed
                  : colors.glass,
              },
            ]
          : [
              styles.medium,
              background && {
                backgroundColor: disabled
                  ? colors.black30
                  : selected || pressed
                  ? colors.black60
                  : colors.black40,
              },
            ]
      }
    >
      {pressed => (
        <Icon
          name={icon}
          size={big ? 26 : iconSize.iconButton}
          color={iconColor(pressed)}
        />
      )}
    </PressableBase>
  );
}

const styles = StyleSheet.create({
  big: {
    width: u(47),
    height: u(48),
    borderRadius: u(8),
    alignItems: 'center',
    justifyContent: 'center',
  },
  medium: {
    width: u(27),
    height: u(27),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
  },
  outline: {
    width: u(27),
    height: u(27),
    borderRadius: u(4),
    borderWidth: border.thin,
    alignItems: 'center',
    justifyContent: 'center',
  },
  primary: {
    width: u(27),
    height: u(27),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
  },
  primaryBig: {
    width: u(30),
    height: u(30),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
  },
});
