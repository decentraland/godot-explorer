import { useState } from 'react';
import { LayoutChangeEvent, StyleSheet, View } from 'react-native';
import { Dropdown, DropdownOption } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { dropdownDirection } from './logic';

const FIELD_DP = 40;
const ROW_DP = 38;
const MAX_ROWS = 5;

/** Dropdown that opens upward when its list would run under the footer, and rises above later fields while open. */
export function FieldDropdown({
  label,
  value,
  options,
  onChange,
  scrollY,
  visibleBottom,
  open,
  onOpenChange,
}: {
  label: string;
  value: string | null;
  options: DropdownOption[];
  onChange: (value: string) => void;
  /** Scroll offset and height of the visible area; the list flips up past `scrollY + visibleBottom`. */
  scrollY: number;
  visibleBottom: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [top, setTop] = useState(0);
  const listHeight = u(ROW_DP) * Math.min(options.length, MAX_ROWS) + u(8);
  const direction = dropdownDirection(
    top + u(FIELD_DP),
    scrollY + visibleBottom,
    listHeight,
  );
  return (
    <View
      style={open ? styles.raised : undefined}
      onLayout={(e: LayoutChangeEvent) => setTop(e.nativeEvent.layout.y)}
    >
      <Dropdown
        label={label}
        value={value}
        options={options}
        onChange={onChange}
        placeholder={t('PROFILE_SELECT')}
        direction={direction}
        defaultOpen={open}
        onOpenChange={onOpenChange}
      />
    </View>
  );
}

const styles = StyleSheet.create({ raised: { zIndex: 50 } });
