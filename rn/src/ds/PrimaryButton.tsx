import { ActivityIndicator, Pressable, StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { colors, radii } from './tokens';
import { u } from './scale';

export type ButtonTone = 'primary' | 'secondary' | 'ghost';

export function PrimaryButton({
  label,
  onPress,
  tone = 'primary',
  disabled = false,
  loading = false,
}: {
  label: string;
  onPress: () => void;
  tone?: ButtonTone;
  disabled?: boolean;
  loading?: boolean;
}) {
  const inactive = disabled || loading;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ disabled: inactive }}
      disabled={inactive}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        tone === 'primary' && styles.primary,
        tone === 'secondary' && styles.secondary,
        inactive && styles.disabled,
        pressed && styles.pressed,
      ]}
    >
      {loading ? (
        <ActivityIndicator color={colors.snow} />
      ) : (
        <AppText
          variant="button"
          style={tone === 'ghost' && { color: colors.gray }}
        >
          {label}
        </AppText>
      )}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    minHeight: u(48),
    paddingHorizontal: u(24),
    borderRadius: radii.pill,
    alignItems: 'center',
    justifyContent: 'center',
  },
  primary: { backgroundColor: colors.ruby },
  secondary: {
    backgroundColor: colors.white10,
    borderWidth: 1,
    borderColor: colors.white30,
  },
  disabled: { opacity: 0.5 },
  pressed: { opacity: 0.85 },
});
