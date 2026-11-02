import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors, fonts } from './tokens';
import { u } from './scale';

export function WarningBanner({
  message,
  icon = 'warning',
  outlined = false,
}: {
  message: string;
  icon?: IconName;
  /** Settings look: lavender 1dp border, plum fill, white icon. */
  outlined?: boolean;
}) {
  return (
    <View
      style={[styles.banner, outlined && styles.outlined]}
      accessibilityRole="alert"
    >
      <Icon
        name={icon}
        size={outlined ? 24 : 22}
        color={outlined ? colors.snow : colors.orange}
      />
      <AppText
        variant="body"
        style={[styles.text, outlined && styles.outlinedText]}
      >
        {message}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  banner: {
    minHeight: u(43),
    padding: u(12),
    borderRadius: u(8),
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.black40,
    gap: u(12),
  },
  outlined: {
    borderWidth: 1,
    borderColor: '#B384CC',
    backgroundColor: '#310850',
    borderRadius: u(5),
    minHeight: u(40),
    paddingVertical: u(8),
  },
  outlinedText: { fontSize: u(15) },
  text: { flex: 1, fontSize: u(14), fontFamily: fonts.medium },
});
