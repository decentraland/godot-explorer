import { Pressable, StyleSheet, TextInput, View } from 'react-native';
import { Icon } from './Icon';
import { colors, fonts } from './tokens';
import { u } from './scale';
import { t } from '../i18n';

/** Search-mode header: optional back chevron, input, clear X when there is text; `framed` is the outlined box. */
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
      <View style={[styles.field, framed && styles.framed]}>
        <Icon
          name="search"
          size={framed ? 22 : 18}
          color={framed ? colors.snow : colors.lavender}
        />
        <TextInput
          value={value}
          onChangeText={onChangeText}
          onSubmitEditing={() => onSubmit?.(value)}
          placeholder={placeholder}
          placeholderTextColor={colors.white70}
          returnKeyType="search"
          autoFocus={autoFocus}
          style={styles.input}
        />
        {value.length > 0 && (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_CLEAR_SEARCH')}
            hitSlop={8}
            onPress={() => onChangeText('')}
          >
            <Icon name="close" size={16} color={colors.lavender} />
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
    height: u(40),
    borderRadius: u(20),
    paddingHorizontal: u(14),
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.glass,
  },
  framed: {
    height: u(34),
    borderRadius: u(6),
    borderWidth: 1,
    borderColor: colors.lavender,
    backgroundColor: 'rgba(30,0,60,0.55)',
  },
  input: {
    flex: 1,
    marginHorizontal: u(8),
    padding: 0,
    fontFamily: fonts.medium,
    fontSize: u(15),
    color: colors.snow,
  },
});
