import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { Button } from './Button';
import { colors, fonts, radius } from './tokens';
import { u } from './scale';

/** Figma `EmptyState` 866:372: an optional icon in an outlined Purple 100 badge over a title and body.
 * `lg` for full screens and sheet content, `sm` for side panels and landscape. `actionLabel` is a
 * code-only retry button under the text (the node has none). */
export function EmptyState({
  icon,
  title,
  body,
  size = 'lg',
  actionLabel,
  onAction,
}: {
  icon?: IconName;
  title: string;
  body?: string;
  size?: 'lg' | 'sm';
  actionLabel?: string;
  onAction?: () => void;
}) {
  const small = size === 'sm';
  return (
    <View style={[styles.root, small ? styles.rootSmall : styles.rootLarge]}>
      {icon && (
        <View style={styles.badge}>
          <Icon name={icon} size={50} color={colors.purple100} />
        </View>
      )}
      <View style={[styles.text, small && styles.textSmall]}>
        <AppText style={[styles.title, small && styles.titleSmall]}>
          {title}
        </AppText>
        {body ? (
          <AppText style={[styles.body, small && styles.bodySmall]}>
            {body}
          </AppText>
        ) : null}
      </View>
      {actionLabel && onAction && (
        <View style={styles.action}>
          <Button label={actionLabel} onPress={onAction} />
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'center', alignItems: 'center', gap: u(12) },
  rootLarge: { width: u(312), maxWidth: '100%' },
  rootSmall: { width: u(240), maxWidth: '100%' },
  badge: {
    width: u(60),
    height: u(60),
    alignItems: 'center',
    justifyContent: 'center',
    borderWidth: u(2),
    borderColor: colors.purple100,
    borderRadius: radius['2xl'],
  },
  text: { alignSelf: 'stretch', gap: u(5) },
  textSmall: { gap: u(3) },
  title: {
    fontFamily: fonts.semibold,
    fontSize: u(21),
    color: colors.purple100,
    textAlign: 'center',
  },
  titleSmall: { fontSize: u(14) },
  body: {
    fontSize: u(13),
    lineHeight: u(17),
    color: colors.purple100,
    textAlign: 'center',
  },
  bodySmall: { fontSize: u(12), lineHeight: undefined },
  action: { alignSelf: 'stretch' },
});
