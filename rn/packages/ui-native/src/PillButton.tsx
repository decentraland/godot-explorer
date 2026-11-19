import { StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { NotificationBadge } from './NotificationBadge';
import { PressableBase } from './internal/PressableBase';
import { colors, radii } from './tokens';
import { u } from './scale';

/** Figma's `Pill` — a selectable filter chip. `big` is the Large size (60dp tall, used for the
 * main mode switch, e.g. Wearables/Emotes); the default is Medium (56dp, sub-filters). `color` is
 * the Figma `Color` variant: Lavender (filters) or Ruby (the Backpack header's mode switch).
 * Figma Button/Pill: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=10-293
 */
export function PillButton({
  label,
  icon,
  selected = false,
  big = false,
  color = 'Lavender',
  badge = false,
  onPress,
}: {
  label: string;
  icon?: IconName;
  selected?: boolean;
  /** The Large size (60dp) — default is Medium (56dp). */
  big?: boolean;
  color?: 'Lavender' | 'Ruby';
  badge?: boolean;
  onPress: () => void;
}) {
  const ruby = color === 'Ruby';
  const bg = selected
    ? ruby
      ? colors.ruby
      : colors.lavender
    : ruby
    ? colors.black40
    : colors.white20;
  const textColor = selected
    ? ruby
      ? colors.snow
      : colors.shadow
    : big
    ? colors.pearl
    : colors.white70;
  return (
    <PressableBase
      onPress={onPress}
      accessibilityLabel={label}
      accessibilityState={{ selected }}
      style={() => [
        styles.pill,
        big ? styles.big : styles.medium,
        { backgroundColor: bg },
      ]}
    >
      {icon && <Icon name={icon} size={16} color={textColor} />}
      <AppText
        variant="button"
        style={[big ? styles.bigText : styles.mediumText, { color: textColor }]}
      >
        {label}
      </AppText>
      {badge && <NotificationBadge style={styles.badge} />}
    </PressableBase>
  );
}

const styles = StyleSheet.create({
  pill: {
    position: 'relative',
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(2),
    paddingLeft: u(7),
    paddingRight: u(8),
    borderRadius: radii.pill,
  },
  big: { height: u(30) },
  medium: { height: u(28) },
  bigText: { fontSize: u(13), lineHeight: u(17) },
  mediumText: { fontSize: u(12), lineHeight: u(15) },
  badge: { position: 'absolute', top: -u(2), right: -u(2) },
});
