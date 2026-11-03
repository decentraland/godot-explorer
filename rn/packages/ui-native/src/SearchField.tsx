import { useState } from 'react';
import { Pressable, StyleSheet, TextInput, View } from 'react-native';
import { Icon } from './Icon';
import { colors, fonts } from './tokens';
import { u } from './scale';
import { t } from './i18n';

/** Figma `Input/Search` (540×68: Black 40%, radius 12, `Icon/Base` search 56, Lavender Light
 * border while focused) with the optional back chevron of `Header` SearchOpen; `framed` only
 * widens the gap after the chevron. */
export function SearchField({
  value,
  onChangeText,
  onSubmit,
  onBack,
  placeholder = 'Search',
  autoFocus = false,
  framed = false,
}: {
  value: string;
  onChangeText: (text: string) => void;
  onSubmit?: (text: string) => void;
  onBack?: () => void;
  placeholder?: string;
  autoFocus?: boolean;
  framed?: boolean;
}) {
  const [focused, setFocused] = useState(false);
  return (
    <View style={styles.row}>
      {onBack && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('APP_BACK')}
          hitSlop={8}
          onPress={onBack}
          style={[styles.back, framed && styles.backFramed]}
        >
          <Icon name="chevronLeft" size={22} />
        </Pressable>
      )}
      <View style={[styles.field, focused && styles.focused]}>
        <Icon name="search" size={28} />
        <TextInput
          value={value}
          onChangeText={onChangeText}
          onSubmitEditing={() => onSubmit?.(value)}
          placeholder={placeholder}
          placeholderTextColor={colors.white50}
          returnKeyType="search"
          autoFocus={autoFocus}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          style={styles.input}
        />
        {value.length > 0 && (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_CLEAR_SEARCH')}
            hitSlop={8}
            onPress={() => onChangeText('')}
          >
            <Icon name="close" size={20} />
          </Pressable>
        )}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center' },
  back: { marginRight: u(8) },
  backFramed: { marginRight: u(20) },
  field: {
    flex: 1,
    height: u(34),
    borderRadius: u(6),
    paddingHorizontal: u(5),
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.black40,
    borderWidth: 1,
    borderColor: 'transparent',
  },
  focused: { borderColor: colors.lavender },
  input: {
    flex: 1,
    marginHorizontal: u(4),
    padding: 0,
    fontFamily: fonts.medium,
    fontSize: u(15),
    color: colors.snow,
  },
});
