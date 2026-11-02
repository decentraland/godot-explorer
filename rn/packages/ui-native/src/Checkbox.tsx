import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { colors } from './tokens';
import { u } from './scale';

const SIZES = { lg: 24, sm: 15 } as const;
const TOUCH = 44;

/** Figma `Control/Checkbox`: Black 40% box with a Snow border, Ruby border + fill when checked
 * (every size). The whole row is the touch target, never the box alone. */
export function Checkbox({
  checked,
  onChange,
  label,
  disabled = false,
  size = 'lg',
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  disabled?: boolean;
  size?: 'lg' | 'sm';
}) {
  const box = u(SIZES[size]);
  return (
    <Pressable
      accessibilityRole="checkbox"
      accessibilityLabel={label}
      accessibilityState={{ checked, disabled }}
      disabled={disabled}
      hitSlop={Math.max(0, (u(TOUCH) - box) / 2)}
      onPress={() => onChange(!checked)}
      style={styles.row}
    >
      <View
        style={[
          styles.box,
          { width: box, height: box },
          checked && styles.boxChecked,
          disabled && styles.boxDisabled,
        ]}
      >
        {checked && (
          <View style={[styles.fill, disabled && styles.fillDisabled]} />
        )}
      </View>
      <AppText style={[styles.label, disabled && styles.labelDisabled]}>
        {label}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: u(6) },
  box: {
    borderRadius: u(1),
    borderWidth: 1,
    borderColor: colors.snow,
    backgroundColor: colors.black40,
    padding: '12.5%',
  },
  boxChecked: { borderColor: colors.ruby },
  boxDisabled: {
    backgroundColor: colors.black30,
    borderColor: colors.textDisabled,
  },
  fill: { flex: 1, borderRadius: u(1), backgroundColor: colors.ruby },
  fillDisabled: { backgroundColor: colors.textDisabled },
  label: {
    flexShrink: 1,
    fontSize: u(13),
    lineHeight: u(18),
    color: colors.snow,
  },
  labelDisabled: { color: colors.textDisabled },
});
