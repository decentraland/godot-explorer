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
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
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
  labelRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: u(10),
  },
  label: { fontFamily: fonts.medium, fontSize: u(15), color: colors.snow },
  labelDisabled: { color: colors.white30 },
  box: {
    minHeight: u(34),
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
  counterBelow: { marginTop: u(4) },
  hint: {
    marginTop: u(4),
    fontFamily: fonts.regular,
    fontSize: u(10),
    color: colors.pearl,
  },
  errorText: { color: colors.danger },
});
