import { useRef, useState } from 'react';
import { Pressable, StyleSheet, TextInput, View } from 'react-native';
import { AppText } from './AppText';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** Figma `Input/CodeSlots`: one 84×130 slot per digit of a verification code. The slot being
 * typed gets the Lavender Light border, filled slots a Pearl one, every slot `status/danger`
 * when `error`. A hidden input carries the real text so the OS keyboard and autofill work. */
export function CodeSlots({
  value,
  length,
  onChangeText,
  error = false,
  autoFocus = false,
  accessibilityLabel,
}: {
  value: string;
  length: number;
  onChangeText: (text: string) => void;
  error?: boolean;
  autoFocus?: boolean;
  accessibilityLabel: string;
}) {
  const input = useRef<TextInput>(null);
  const [focused, setFocused] = useState(autoFocus);
  const digits = value.slice(0, length).split('');
  const current = Math.min(digits.length, length - 1);
  return (
    <Pressable
      accessibilityRole="none"
      onPress={() => input.current?.focus()}
      style={styles.row}
    >
      {Array.from({ length }, (_, i) => {
        const digit = digits[i];
        const typing = focused && i === current && !error;
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
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', gap: u(8), justifyContent: 'center' },
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
