import { StyleProp, StyleSheet, View, ViewStyle } from 'react-native';
import { AppText } from './AppText';
import { Button } from './Button';
import { colors, radius, rarityColors, space } from './tokens';
import { u } from './scale';

/** Figma `Notification` (Notifications page, node-id 1171-73): title, copy and one primary action. */
export function Notification({
  title,
  body,
  actionLabel,
  onAction,
  style,
}: {
  title: string;
  body: string;
  actionLabel: string;
  onAction: () => void;
  style?: StyleProp<ViewStyle>;
}) {
  return (
    <View style={[styles.card, style]}>
      <View style={styles.content}>
        <AppText variant="H3">{title}</AppText>
        <AppText variant="Card/Body">{body}</AppText>
      </View>
      <Button label={actionLabel} onPress={onAction} small fullWidth />
    </View>
  );
}

const styles = StyleSheet.create({
  card: {
    gap: space.lg,
    paddingTop: space['20'],
    paddingBottom: space.lg,
    paddingHorizontal: space['26'],
    borderRadius: radius.lg,
    borderWidth: u(0.5),
    borderColor: rarityColors.legendary.light,
    backgroundColor: colors.black20,
  },
  content: { gap: space.xs },
});
