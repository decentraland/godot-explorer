import { useEffect, useRef, useState } from 'react';
import { Animated, StyleSheet, TextInput, View } from 'react-native';
import { AppText } from './AppText';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** The typing caret, blinking at the system's half-second. */
function Caret() {
  const opacity = useRef(new Animated.Value(1)).current;
  useEffect(() => {
    const blink = Animated.loop(
      Animated.sequence([
        Animated.delay(530),
        Animated.timing(opacity, {
          toValue: 0,
          duration: 0,
          useNativeDriver: true,
        }),
        Animated.delay(530),
        Animated.timing(opacity, {
          toValue: 1,
          duration: 0,
          useNativeDriver: true,
        }),
      ]),
    );
    blink.start();
    return () => blink.stop();
  }, [opacity]);
  return <Animated.View style={[styles.caret, { opacity }]} />;
}

/** Figma `Input/CodeSlots` laid out as `OTP-Input`: one 84×130 slot per digit, 10 apart. The slot
 * being typed gets the Lavender Light border, filled slots a Pearl one, every slot `status/danger`
 * when `error`. `errorMessage` reserves its line under the slots and shows it while `error`. A
 * transparent input laid over the slots carries the real text, so a tap lands on it and the OS
 * keyboard and autofill work.
 * Figma Input/CodeSlots: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=562-5058
 */
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
  const [focused, setFocused] = useState(false);
  // Not `autoFocus`: inside a `Modal` Android hides the keyboard when the dialog window takes focus.
  useEffect(() => {
    if (!autoFocus) {
      return;
    }
    const id = setTimeout(() => input.current?.focus(), 300);
    return () => clearTimeout(id);
  }, [autoFocus]);
  const digits = value.slice(0, length).split('');
  const slots = (
    <View style={styles.row}>
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
              typing && <Caret />
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
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        cursorColor="transparent"
        selectionColor="transparent"
        style={styles.input}
      />
    </View>
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
  /** `status/danger` per the States rule (the `OTP-Input` node draws Ruby). */
  message: {
    minHeight: u(12),
    fontSize: u(10),
    color: colors.danger,
    textAlign: 'center',
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
  // Not 'transparent': Android reads an alpha of 0 as unset and draws the text black.
  input: { ...StyleSheet.absoluteFill, color: 'rgba(0,0,0,0.01)' },
});
