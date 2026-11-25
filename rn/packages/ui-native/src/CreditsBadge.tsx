import { Pressable, StyleProp, StyleSheet, ViewStyle } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { color, control, radius, space } from './tokens';
import { t } from './i18n';

/** Figma `Badge/Credits`: the coin and the balance in a dark pill. Tappable only with `onPress`. */
export function CreditsBadge({
  amount,
  onPress,
  style,
}: {
  amount: number;
  onPress?: () => void;
  style?: StyleProp<ViewStyle>;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('APP_CREDITS')}
      disabled={!onPress}
      onPress={onPress}
      style={[styles.badge, style]}
    >
      <Icon name="credits" size={20} />
      <AppText variant="Header/Credits" style={styles.amount}>
        {String(amount)}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  badge: {
    height: control['pill-lg'],
    paddingHorizontal: space['18'],
    gap: space['10'],
    flexDirection: 'row',
    alignItems: 'center',
    borderRadius: radius.pill,
    backgroundColor: color.surface.overlay,
  },
  amount: { color: color.text.primary },
});
