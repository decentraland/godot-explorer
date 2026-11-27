import { StyleProp, StyleSheet, Text, View, ViewStyle } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors, fonts, radius, space } from './tokens';
import { u } from './scale';

export type AlertBackground = 'solid' | 'translucent';

/**
 * Figma `Alert` (Notifications page, node-id 1171-72). `solid` (Deep Plum) sits over text or busy
 * content so it stays readable; `translucent` (Black 30) sits on a plain background.
 */
export function Alert({
  message,
  lead,
  icon = 'warning',
  background = 'solid',
  style,
}: {
  message: string;
  /** Bold sentence before the message ("Reward claim failed."). */
  lead?: string;
  icon?: IconName;
  background?: AlertBackground;
  style?: StyleProp<ViewStyle>;
}) {
  return (
    <View
      style={[
        styles.box,
        background === 'solid' ? styles.solid : styles.translucent,
        style,
      ]}
      accessibilityRole="alert"
    >
      <Icon name={icon} size={u(27)} color={colors.snow} />
      <AppText variant="Alert/Body" style={styles.text}>
        {!!lead && <Text style={styles.lead}>{lead} </Text>}
        {message}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  box: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: space.lg,
    paddingVertical: space.md,
    paddingHorizontal: space.lg,
    borderRadius: radius.lg,
    borderWidth: u(0.5),
    borderColor: colors.lavender70,
  },
  solid: { backgroundColor: colors.menu },
  translucent: { backgroundColor: colors.black30 },
  text: { flex: 1 },
  lead: { fontFamily: fonts.bold },
});
