import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { colors } from './tokens';
import { u } from './scale';

export function ReadOnlyField({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <View style={styles.root}>
      <AppText variant="body" style={styles.label}>
        {label}
      </AppText>
      <View
        style={styles.box}
        accessible
        accessibilityLabel={`${label}: ${value}`}
      >
        <AppText variant="body" numberOfLines={1}>
          {value}
        </AppText>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'stretch' },
  label: { fontSize: u(15), marginBottom: u(6) },
  box: {
    height: u(34),
    borderRadius: u(5),
    paddingHorizontal: u(12),
    justifyContent: 'center',
    backgroundColor: colors.fieldDark,
  },
});
