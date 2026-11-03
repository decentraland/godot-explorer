import { ReactNode, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors, elevation, fonts } from './tokens';
import { u } from './scale';

/** `icon` is drawn after the label (a claimed-name seal). */
export type DropdownOption = {
  value: string;
  label: string;
  icon?: IconName;
  iconColor?: string;
  /** Drawn instead of `icon` (a custom badge). */
  iconNode?: ReactNode;
};

const FIELD_HEIGHT = 34;
const LIST_GAP = 2;

/** Figma `Input/DropdownLabeled`: label + `Input/Dropdown` field + an overlay `Input/OptionsList`
 * (it does not push content) that opens down or up. `readOnly` has no chevron and cannot open;
 * `error` is the Red border plus the message below. */
export function Dropdown({
  label,
  value,
  options,
  onChange,
  placeholder = 'Select',
  helper,
  error,
  disabled = false,
  readOnly = false,
  direction = 'down',
  maxVisibleRows = 5,
  defaultOpen = false,
  onOpenChange,
}: {
  label: string;
  value: string | null;
  options: DropdownOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  helper?: string;
  error?: string;
  disabled?: boolean;
  readOnly?: boolean;
  direction?: 'down' | 'up';
  maxVisibleRows?: number;
  defaultOpen?: boolean;
  /** Lets a parent raise this field above its siblings while the list is open. */
  onOpenChange?: (open: boolean) => void;
}) {
  const [open, setOpenState] = useState(defaultOpen);
  const setOpen = (next: boolean) => {
    setOpenState(next);
    onOpenChange?.(next);
  };
  const selected = options.find(o => o.value === value);
  const locked = disabled || readOnly;
  const isOpen = open && !locked;
  const rows = Math.min(options.length, maxVisibleRows);
  const listHeight = u(FIELD_HEIGHT) * rows + u(4) * (rows - 1);

  const list = isOpen && (
    <View
      style={[
        styles.list,
        { maxHeight: listHeight + u(8) },
        direction === 'up' ? styles.listUp : styles.listDown,
      ]}
    >
      <ScrollView nestedScrollEnabled showsVerticalScrollIndicator={false}>
        {options.map(option => {
          const isSelected = option.value === value;
          return (
            <Pressable
              key={option.value}
              accessibilityRole="menuitem"
              accessibilityLabel={option.label}
              accessibilityState={{ selected: isSelected }}
              onPress={() => {
                setOpen(false);
                onChange(option.value);
              }}
              style={[styles.row, isSelected && styles.rowSelected]}
            >
              <AppText
                numberOfLines={1}
                style={[styles.text, isSelected && styles.textSelected]}
              >
                {option.label}
              </AppText>
              {(option.iconNode || option.icon) && (
                <View style={styles.optionIcon}>
                  {option.iconNode ?? (
                    <Icon
                      name={option.icon!}
                      size={16}
                      color={option.iconColor}
                    />
                  )}
                </View>
              )}
              <View style={styles.spacer} />
              {isSelected && (
                <Icon name="check" size={20} color={colors.lavender} />
              )}
            </Pressable>
          );
        })}
      </ScrollView>
    </View>
  );

  return (
    <View style={[styles.root, isOpen && styles.rootOpen]}>
      {!!label && (
        <AppText style={[styles.label, disabled && styles.labelDisabled]}>
          {label}
        </AppText>
      )}
      <View>
        {direction === 'up' && list}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={label}
          accessibilityState={{ expanded: isOpen, disabled: locked }}
          disabled={locked}
          onPress={() => setOpen(!open)}
          style={[
            styles.field,
            isOpen && styles.fieldOpen,
            !!error && styles.fieldError,
            disabled && styles.fieldDisabled,
          ]}
        >
          <AppText
            numberOfLines={1}
            style={[
              styles.text,
              !selected && styles.placeholder,
              readOnly && styles.textReadOnly,
              disabled && styles.textDisabled,
            ]}
          >
            {selected ? selected.label : placeholder}
          </AppText>
          {(selected?.iconNode || selected?.icon) && (
            <View style={styles.optionIcon}>
              {selected.iconNode ?? (
                <Icon
                  name={selected.icon!}
                  size={16}
                  color={selected.iconColor}
                />
              )}
            </View>
          )}
          <View style={styles.spacer} />
          {!readOnly && (
            <Icon
              name={isOpen ? 'chevronUp' : 'chevronDown'}
              size={20}
              color={disabled ? colors.white20 : colors.snow}
            />
          )}
        </Pressable>
        {direction === 'down' && list}
      </View>
      {(error || helper) && (
        <AppText style={[styles.helper, !!error && styles.errorText]}>
          {error ?? helper}
        </AppText>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'stretch' },
  rootOpen: { zIndex: 10, elevation: 10 },
  label: {
    fontFamily: fonts.medium,
    fontSize: u(15),
    color: colors.snow,
    marginBottom: u(10),
  },
  labelDisabled: { color: colors.white30 },
  field: {
    height: u(FIELD_HEIGHT),
    borderRadius: u(6),
    paddingHorizontal: u(8),
    gap: u(5),
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.black40,
    borderWidth: 1,
    borderColor: 'transparent',
  },
  fieldOpen: { borderColor: colors.lavender },
  fieldError: { borderColor: colors.danger },
  fieldDisabled: { backgroundColor: colors.black20 },
  text: {
    fontFamily: fonts.medium,
    fontSize: u(15),
    color: colors.snow,
    flexShrink: 1,
  },
  placeholder: { color: colors.white50 },
  textReadOnly: { color: colors.gray },
  textDisabled: { color: colors.white20 },
  textSelected: { color: colors.lavender },
  optionIcon: { marginLeft: u(4) },
  spacer: { flex: 1 },
  helper: {
    marginTop: u(5),
    paddingBottom: u(4),
    fontSize: u(11),
    color: colors.pearl,
  },
  errorText: { color: colors.danger },
  list: {
    position: 'absolute',
    left: 0,
    right: 0,
    zIndex: 20,
    borderRadius: u(6),
    borderWidth: 1,
    borderColor: colors.lavender70,
    backgroundColor: colors.menu,
    padding: u(4),
    gap: u(4),
  },
  listDown: { top: u(FIELD_HEIGHT + LIST_GAP), ...elevation.down },
  listUp: { bottom: u(FIELD_HEIGHT + LIST_GAP), ...elevation.up },
  row: {
    height: u(FIELD_HEIGHT),
    borderRadius: u(6),
    paddingHorizontal: u(8),
    flexDirection: 'row',
    alignItems: 'center',
  },
  rowSelected: { backgroundColor: colors.black40 },
});
