import { useRef, useState } from 'react';
import { Pressable, StyleSheet, TextInput, View } from 'react-native';
import { AppText } from './AppText';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** Figma `Input/CodeSlots` laid out as `OTP-Input`: one 84×130 slot per digit, 10 apart. The slot
 * being typed gets the Lavender Light border, filled slots a Pearl one, every slot `status/danger`
 * when `error`. `errorMessage` reserves its line under the slots and shows it while `error`. A
 * hidden input carries the real text so the OS keyboard and autofill work. */
export function CodeSlots({
  value,
  length,
  onChangeText,
  error = false,
  errorMessage,
  autoFocus = false,
  accessibilityLabel,
}: {
  value: string;
  length: number;
  onChangeText: (text: string) => void;
  error?: boolean;
  errorMessage?: string;
  autoFocus?: boolean;
  accessibilityLabel: string;
}) {
  const input = useRef<TextInput>(null);
  const [focused, setFocused] = useState(autoFocus);
  const digits = value.slice(0, length).split('');
  const slots = (
    <Pressable
      accessibilityRole="none"
      onPress={() => input.current?.focus()}
      style={styles.row}
    >
      {Array.from({ length }, (_, i) => {
        const digit = digits[i];
        const typing = focused && i === digits.length && !error;
        return (
          <View
            key={i}
            style={[
              styles.slot,
              digit !== undefined && styles.slotFilled,
              typing && styles.slotTyping,
              error && styles.slotError,
            ]}
          >
            {digit !== undefined ? (
              <AppText
                variant="heading"
                style={[styles.digit, error && styles.digitError]}
              >
                {digit}
              </AppText>
            ) : (
              typing && <View style={styles.caret} />
            )}
          </View>
        );
      })}
      <TextInput
        ref={input}
        accessibilityLabel={accessibilityLabel}
        value={value}
        onChangeText={text =>
          onChangeText(text.replace(/\D/g, '').slice(0, length))
        }
        keyboardType="number-pad"
        textContentType="oneTimeCode"
        autoComplete="one-time-code"
        maxLength={length}
        autoFocus={autoFocus}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        caretHidden
        style={styles.hidden}
      />
    </Pressable>
  );
  if (errorMessage === undefined) {
    return slots;
  }
  return (
    <View style={styles.withMessage}>
      {slots}
      <AppText style={styles.message}>{error ? errorMessage : ''}</AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', gap: u(5), justifyContent: 'center' },
  withMessage: { gap: u(4) },
  /** `OTP-Input`'s message is Ruby and Title Case on the node, not the States page's `status/danger`. */
  message: {
    minHeight: u(12),
    fontSize: u(10),
    color: colors.ruby,
    textAlign: 'center',
    textTransform: 'capitalize',
  },
  slot: {
    width: u(42),
    height: u(65),
    borderRadius: u(8),
    borderWidth: 1,
    borderColor: colors.shadow,
    backgroundColor: colors.black40,
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
  },
  slotFilled: { borderColor: colors.pearl },
  slotTyping: { borderColor: colors.lavender, backgroundColor: colors.black70 },
  slotError: { borderColor: colors.danger },
  digit: { fontFamily: fonts.semibold, fontSize: u(24), color: colors.snow },
  digitError: { color: colors.danger },
  caret: { width: u(1.66), height: u(25), backgroundColor: colors.white },
  hidden: {
    position: 'absolute',
    opacity: 0,
    width: 1,
    height: 1,
  },
});
