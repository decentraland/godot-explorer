import { StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, IconName, u } from '../ds';

/** Centered icon in a rounded outline, title and a two-line body (Social and Notifications states). */
export function PanelEmpty({
  icon,
  title,
  body,
}: {
  icon: IconName;
  title: string;
  body?: string;
}) {
  return (
    <View style={styles.root}>
      <View style={styles.box}>
        <Icon name={icon} size={30} color={colors.lavender} strokeWidth={1.6} />
      </View>
      <AppText variant="title" style={styles.title}>
        {title}
      </AppText>
      {!!body && (
        <AppText variant="body" style={styles.body}>
          {body}
        </AppText>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, alignItems: 'center', paddingTop: u(48) },
  box: {
    width: u(60),
    height: u(60),
    borderRadius: u(12),
    borderWidth: 1.5,
    borderColor: 'rgba(232,185,255,0.8)',
    alignItems: 'center',
    justifyContent: 'center',
  },
  title: { fontSize: u(16), marginTop: u(12), color: colors.snow },
  body: {
    fontSize: u(12),
    lineHeight: u(15),
    marginTop: u(6),
    color: colors.white70,
    textAlign: 'center',
    maxWidth: u(190),
  },
});
