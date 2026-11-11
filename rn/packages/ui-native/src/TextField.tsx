import { ReactNode, useState } from 'react';
import { Pressable, StyleSheet, TextInput, View } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { colors, fonts } from './tokens';
import { u } from './scale';
import { t } from './i18n';

export const counterLabel = (length: number, max: number): string =>
  `${length}/${max}`;

/** Figma `Input/Labeled` + `Input/Field`: label, optional counter, clear X while focused, and the
 * Focus (Lavender Light) / Error (`status/danger`) borders. Disabled is a palette state, not a dim. */
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
  onFocusChange,
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
  /** Counter under the box, on one row with the helper or error (the Name field). */
  counterBelow?: boolean;
  /** Helper under the label instead of under the box (an error still shows under the box). */
  helperAbove?: boolean;
  /** Static content between the text and the clear X (a `#1234` tag). */
  suffix?: ReactNode;
  /** Height in dp of a multiline box. */
  multilineHeight?: number;
  autoFocus?: boolean;
  secureTextEntry?: boolean;
  /** Told when the field gains or loses focus, for a caller that validates on blur. */
  onFocusChange?: (focused: boolean) => void;
}) {
  const [focused, setFocused] = useState(false);
  const showClear = focused && value.length > 0 && !disabled && !!onClear;
  const hint = error ?? (helperAbove ? undefined : helper);
  const counter =
    showCounter && maxLength !== undefined ? (
      <AppText
        variant="caption"
        style={[
          styles.counter,
          disabled && styles.textDisabled,
          !!error && styles.errorText,
          !error && value.length >= maxLength && styles.counterMax,
        ]}
      >
        {counterLabel(value.length, maxLength)}
      </AppText>
    ) : null;
  return (
    <View style={styles.root}>
      {(!!label || (!counterBelow && !!counter)) && (
        <View style={styles.labelRow}>
          <AppText
            variant="body"
            style={[styles.label, disabled && styles.labelDisabled]}
          >
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
          multiline &&
            multilineHeight !== undefined && {
              minHeight: u(multilineHeight),
            },
          focused && styles.focused,
          !!error && styles.error,
          disabled && styles.boxDisabled,
        ]}
      >
        <TextInput
          accessibilityLabel={label}
          value={value}
          onChangeText={onChangeText}
          placeholder={placeholder}
          placeholderTextColor={disabled ? colors.white20 : colors.white50}
          multiline={multiline}
          maxLength={maxLength}
          editable={!disabled}
          autoFocus={autoFocus}
          secureTextEntry={secureTextEntry}
          autoCapitalize={autoCapitalize}
          keyboardType={keyboardType}
          onFocus={() => {
            setFocused(true);
            onFocusChange?.(true);
          }}
          onBlur={() => {
            setFocused(false);
            onFocusChange?.(false);
          }}
          style={[
            styles.input,
            disabled && styles.textDisabled,
            multiline && styles.inputMultiline,
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
            <Icon name="close" size={20} />
          </Pressable>
        )}
      </View>
      {counterBelow && counter ? (
        <View style={styles.descriptionRow}>
          {hint ? (
            <AppText
              variant="caption"
              style={[
                styles.hint,
                styles.hintInRow,
                !!error && styles.errorText,
              ]}
            >
              {hint}
            </AppText>
          ) : (
            <View />
          )}
          {counter}
        </View>
      ) : (
        hint && (
          <AppText
            variant="caption"
            style={[styles.hint, !!error && styles.errorText]}
          >
            {hint}
          </AppText>
        )
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'stretch' },
  labelRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: u(9.5),
  },
  /** Figma `Input-Label`: Medium 30 px in a 36 px line, so Android and web agree on the baseline. */
  label: {
    fontFamily: fonts.medium,
    fontSize: u(15),
    lineHeight: u(18),
    color: colors.snow,
  },
  labelDisabled: { color: colors.white30 },
  box: {
    minHeight: u(35),
    borderRadius: u(6),
    paddingHorizontal: u(8),
    gap: u(5),
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.black40,
    borderWidth: 1,
    borderColor: 'transparent',
  },
  boxMultiline: {
    alignItems: 'flex-start',
    paddingVertical: u(8),
    minHeight: u(108),
  },
  boxDisabled: { backgroundColor: colors.black30 },
  focused: { borderColor: colors.lavender },
  error: { borderColor: colors.danger },
  input: {
    flex: 1,
    padding: 0,
    fontFamily: fonts.medium,
    fontSize: u(15),
    color: colors.snow,
  },
  textDisabled: { color: colors.white20 },
  inputMultiline: { alignSelf: 'stretch', textAlignVertical: 'top' },
  helperAbove: {
    marginBottom: u(5),
    paddingBottom: u(4),
    fontFamily: fonts.regular,
    fontSize: u(11),
    color: colors.pearl,
  },
  counter: { fontFamily: fonts.regular, fontSize: u(10), color: colors.pearl },
  counterMax: { color: colors.ruby200 },
  descriptionRow: {
    marginTop: u(2),
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: u(8),
  },
  hint: {
    marginTop: u(4),
    fontFamily: fonts.regular,
    fontSize: u(10),
    lineHeight: u(12),
    color: colors.pearl,
  },
  hintInRow: { marginTop: 0, flexShrink: 1 },
  errorText: { color: colors.danger },
});
