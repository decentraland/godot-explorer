import { StyleSheet } from 'react-native';
import { Icon, IconName } from './Icon';
import { NotificationBadge } from './NotificationBadge';
import { PressableBase } from './internal/PressableBase';
import { colors } from './tokens';
import { u } from './scale';

/** Figma's icon-only button family — `variant="filled"` covers both `Button/Discover` (`big`,
 * 94×96, purple overlay) and `Button/Filter` (default, 60×60, black overlay); `variant="outline"`
 * is `Button/Social` (54×54, 2px border, used for accept/cancel-style circular actions). Colors
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
  badge = false,
  /** `variant="outline"` only: the border colour while pressed, for a semantic accept/cancel
   * flash (e.g. `colors.ruby` for cancel, a green rarity colour for accept). Defaults to no
   * change — only the background darkens on press. */
  accentColor,
  accessibilityLabel,
}: {
  icon: IconName;
  onPress: () => void;
  variant?: 'filled' | 'outline';
  /** `variant="filled"` only: the Large Discover size (94×96) — default is the Filter size (60×60). */
  big?: boolean;
  selected?: boolean;
  disabled?: boolean;
  badge?: boolean;
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
              ? 'rgba(160,155,168,0.5)'
              : pressed
              ? accentColor ?? colors.snow
              : colors.snow,
          },
        ]}
      >
        <Icon name={icon} size={20} color={colors.snow} />
        {badge && <NotificationBadge style={styles.outlineBadge} />}
      </PressableBase>
    );
  }
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
                backgroundColor: `rgba(105,31,169,${
                  disabled ? 0.2 : selected ? 0.7 : pressed ? 0.3 : 0.4
                })`,
              },
            ]
          : [
              styles.medium,
              // Figma's Disabled swatch for Filter reuses the Default background, so there is
              // no disabled style here.
              {
                backgroundColor:
                  selected || pressed ? colors.black60 : colors.black40,
              },
            ]
      }
    >
      <Icon name={icon} size={big ? 26 : 21} color={colors.snow} />
      {badge && (
        <NotificationBadge
          style={big ? styles.bigBadge : styles.mediumBadge}
        />
      )}
    </PressableBase>
  );
}

const styles = StyleSheet.create({
  big: {
    position: 'relative',
    width: u(47),
    height: u(48),
    borderRadius: u(8),
    alignItems: 'center',
    justifyContent: 'center',
  },
  medium: {
    position: 'relative',
    width: u(30),
    height: u(30),
    borderRadius: u(5),
    alignItems: 'center',
    justifyContent: 'center',
  },
  outline: {
    position: 'relative',
    width: u(27),
    height: u(27),
    borderRadius: u(4),
    borderWidth: 2,
    alignItems: 'center',
    justifyContent: 'center',
  },
  bigBadge: { position: 'absolute', top: -u(2), right: -u(2) },
  mediumBadge: { position: 'absolute', top: -u(2), right: u(1) },
  outlineBadge: { position: 'absolute', top: -u(4), right: -u(4) },
});
