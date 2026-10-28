import { ReactNode, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors } from './tokens';
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

const ROW_HEIGHT = 38;

/** Field plus an overlay list (it does not push content) that opens down or up, with a check on the selected row. */
export function Dropdown({
  label,
  value,
  options,
  onChange,
  placeholder = 'Select',
  helper,
  disabled = false,
  direction = 'down',
  maxVisibleRows = 5,
  defaultOpen = false,
  onOpenChange,
  variant = 'default',
}: {
  label: string;
  value: string | null;
  options: DropdownOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  helper?: string;
  disabled?: boolean;
  direction?: 'down' | 'up';
  maxVisibleRows?: number;
  defaultOpen?: boolean;
  /** Lets a parent raise this field above its siblings while the list is open. */
  onOpenChange?: (open: boolean) => void;
  /** `settings`: tighter field padding, label gap and helper copy. */
  variant?: 'default' | 'settings';
}) {
  const tight = variant === 'settings';
  const [open, setOpenState] = useState(defaultOpen);
  const setOpen = (next: boolean) => {
    setOpenState(next);
    onOpenChange?.(next);
  };
  const selected = options.find(o => o.value === value);
  const isOpen = open && !disabled;
  const listHeight = u(ROW_HEIGHT) * Math.min(options.length, maxVisibleRows);

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
                variant="body"
                style={[styles.rowText, isSelected && styles.rowTextSelected]}
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
                <Icon name="check" size={16} color={colors.lavender} />
              )}
            </Pressable>
          );
        })}
      </ScrollView>
    </View>
  );

  return (
    <View
      style={[
        styles.root,
        isOpen && styles.rootOpen,
        disabled && styles.disabled,
      ]}
    >
      {!!label && (
        <AppText
          variant="body"
          style={[styles.label, tight && styles.labelTight]}
        >
          {label}
        </AppText>
      )}
      <View>
        {direction === 'up' && list}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={label}
          accessibilityState={{ expanded: isOpen, disabled }}
          disabled={disabled}
          onPress={() => setOpen(!open)}
          style={[
            styles.field,
            tight && styles.fieldTight,
            isOpen && styles.fieldOpen,
          ]}
        >
          <AppText
            variant="body"
            numberOfLines={1}
            style={[styles.fieldText, !selected && styles.placeholder]}
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
          <Icon name={isOpen ? 'chevronUp' : 'chevronDown'} size={20} />
        </Pressable>
        {direction === 'down' && list}
      </View>
      {helper && (
        <AppText
          variant="caption"
          style={[styles.helper, tight && styles.helperTight]}
        >
          {helper}
        </AppText>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { alignSelf: 'stretch' },
  rootOpen: { zIndex: 10, elevation: 10 },
  disabled: { opacity: 0.35 },
  label: { fontSize: u(15), marginBottom: u(6) },
  field: {
    height: u(34),
    borderRadius: u(5),
    paddingHorizontal: u(12),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: colors.fieldDark,
    borderWidth: 1,
    borderColor: 'transparent',
  },
  labelTight: { marginBottom: u(9) },
  fieldTight: { paddingHorizontal: u(8) },
  helperTight: { marginTop: u(3), fontSize: u(10) },
  fieldOpen: { borderColor: colors.lavender },
  fieldText: { fontSize: u(15), flexShrink: 1 },
  optionIcon: { marginLeft: u(6) },
  spacer: { flex: 1 },
  placeholder: { opacity: 0.5 },
  helper: { marginTop: u(6), color: colors.lavender70, fontSize: u(11) },
  list: {
    position: 'absolute',
    left: 0,
    right: 0,
    zIndex: 20,
    elevation: 20,
    borderRadius: u(5),
    borderWidth: 1,
    borderColor: colors.purple350,
    backgroundColor: colors.panelDark,
    padding: u(4),
  },
  listDown: { top: u(38) },
  listUp: { bottom: u(38) },
  row: {
    height: u(ROW_HEIGHT),
    borderRadius: u(5),
    paddingHorizontal: u(12),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  rowSelected: { backgroundColor: colors.purple550 },
  rowText: { fontSize: u(15) },
  rowTextSelected: { color: colors.lavender },
});
