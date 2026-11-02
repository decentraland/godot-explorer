import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { colors } from './tokens';
import { u } from './scale';

export const MAX_BADGE = 99;

/** Ruby notification dot; with a `count` it grows into a number pill (capped at 99+). */
export function Badge({ count }: { count?: number }) {
  if (count === undefined) {
    return <View style={styles.dot} />;
  }
  const label = count > MAX_BADGE ? `${MAX_BADGE}+` : String(count);
  return (
    <View style={styles.count}>
      <AppText variant="caption" style={styles.countText}>
        {label}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  dot: {
    width: u(8),
    height: u(8),
    borderRadius: u(4),
    backgroundColor: colors.ruby,
  },
  count: {
    minWidth: u(16),
    height: u(16),
    paddingHorizontal: u(4),
    borderRadius: u(8),
    backgroundColor: colors.ruby,
    alignItems: 'center',
    justifyContent: 'center',
  },
  countText: { fontSize: u(10), lineHeight: u(12), color: colors.snow },
});
