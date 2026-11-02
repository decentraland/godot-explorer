import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { PrimaryButton } from './PrimaryButton';
import { colors } from './tokens';
import { u } from './scale';

/** Centered icon + title + body, with an optional action (no results, connection lost, ...). */
export function EmptyState({
  icon,
  title,
  body,
  actionLabel,
  onAction,
}: {
  icon: IconName;
  title: string;
  body?: string;
  actionLabel?: string;
  onAction?: () => void;
}) {
  return (
    <View style={styles.root}>
      <Icon name={icon} size={48} color={colors.lavender} strokeWidth={1.6} />
      <AppText variant="title" style={styles.title}>
        {title}
      </AppText>
      {body && (
        <AppText variant="body" style={styles.body}>
          {body}
        </AppText>
      )}
      {actionLabel && onAction && (
        <View style={styles.action}>
          <PrimaryButton label={actionLabel} onPress={onAction} />
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    alignItems: 'center',
    paddingHorizontal: u(32),
    paddingVertical: u(24),
  },
  title: { marginTop: u(12), textAlign: 'center', fontSize: u(18) },
  body: {
    marginTop: u(6),
    textAlign: 'center',
    fontSize: u(13),
    color: colors.white70,
  },
  action: { marginTop: u(16), alignSelf: 'stretch' },
});
