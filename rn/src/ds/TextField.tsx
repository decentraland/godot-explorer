import { ReactNode, useState } from 'react';
import { Pressable, StyleSheet, TextInput, View } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { colors, fonts } from './tokens';
import { u } from './scale';
import { t } from '../i18n';

export const counterLabel = (length: number, max: number): string =>
  `${length}/${max}`;

/** Labelled input with optional counter, clear X while focused, and focus / error / disabled borders. */
export function TextField({
  label,
  value,
  onChangeText,
  placeholder,
  multiline = false,
  maxLength,
  showCounter = false,
  error,
  helper,
  disabled = false,
  onClear,
  autoCapitalize,
  keyboardType,
  counterBelow = false,
  helperAbove = false,
  suffix,
  multilineHeight,
  autoFocus = false,
  secureTextEntry = false,
}: {
  label: string;
  value: string;
  onChangeText: (text: string) => void;
  placeholder?: string;
  multiline?: boolean;
  maxLength?: number;
  showCounter?: boolean;
  error?: string;
  helper?: string;
  disabled?: boolean;
  onClear?: () => void;
  autoCapitalize?: 'none' | 'sentences' | 'words' | 'characters';
  keyboardType?: 'default' | 'url' | 'email-address' | 'numeric';
  /** Counter under the box instead of beside the label. */
  counterBelow?: boolean;
  /** Helper under the label instead of under the box (an error still shows under the box). */
  helperAbove?: boolean;
  /** Static content between the text and the clear X (a `#1234` tag). */
  suffix?: ReactNode;
  /** Height in dp of a multiline box. */
  multilineHeight?: number;
  autoFocus?: boolean;
  secureTextEntry?: boolean;
}) {
  const [focused, setFocused] = useState(false);
  const showClear = focused && value.length > 0 && !disabled && !!onClear;
  const hint = error ?? (helperAbove ? undefined : helper);
  const counter =
    showCounter && maxLength !== undefined ? (
      <AppText variant="caption">
        {counterLabel(value.length, maxLength)}
      </AppText>
    ) : null;
  return (
    <View style={[styles.root, disabled && styles.disabled]}>
      {(!!label || (!counterBelow && !!counter)) && (
        <View style={styles.labelRow}>
          <AppText variant="body" style={styles.label}>
            {label}
          </AppText>
          {!counterBelow && counter}
        </View>
      )}
      {helperAbove && !!helper && (
        <AppText variant="caption" style={styles.helperAbove}>
          {helper}
        </AppText>
      )}
      <View
        style={[
          styles.box,
          multiline && styles.boxMultiline,
          focused && styles.focused,
          !!error && styles.error,
        ]}
      >
        <TextInput
          accessibilityLabel={label}
          value={value}
          onChangeText={onChangeText}
          placeholder={placeholder}
          placeholderTextColor={colors.white30}
          multiline={multiline}
          maxLength={maxLength}
          editable={!disabled}
          autoFocus={autoFocus}
          secureTextEntry={secureTextEntry}
          autoCapitalize={autoCapitalize}
          keyboardType={keyboardType}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          style={[
            styles.input,
            multiline && styles.inputMultiline,
            multiline &&
              multilineHeight !== undefined && {
                minHeight: u(multilineHeight),
              },
          ]}
        />
        {suffix}
        {showClear && (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_CLEAR')}
            hitSlop={8}
            onPress={onClear}
          >
            <Icon name="close" size={16} color={colors.lavender} />
          </Pressable>
        )}
      </View>
      {hint && (
        <AppText
          variant="caption"
          style={[styles.hint, !!error && styles.errorText]}
        >
          {hint}
        </AppText>
      )}
      {counterBelow && counter && (
        <View style={styles.counterBelow}>{counter}</View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'stretch' },
  disabled: { opacity: 0.5 },
  labelRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: u(6),
  },
  label: { fontSize: u(15) },
  box: {
    minHeight: u(34),
    borderRadius: u(6),
    paddingHorizontal: u(12),
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.black40,
    borderWidth: 1,
    borderColor: 'transparent',
  },
  boxMultiline: { alignItems: 'flex-start', paddingVertical: u(8) },
  focused: { borderColor: '#D9B3FF' },
  error: { borderColor: colors.ruby },
  input: {
    flex: 1,
    padding: 0,
    fontFamily: fonts.medium,
    fontSize: u(15),
    color: colors.snow,
  },
  inputMultiline: { minHeight: u(80), textAlignVertical: 'top' },
  helperAbove: { marginBottom: u(6), fontSize: u(10), color: colors.pearl },
  counterBelow: { marginTop: u(6) },
  hint: { marginTop: u(6), fontSize: u(11), color: colors.lavender70 },
  errorText: { color: colors.ruby },
});
