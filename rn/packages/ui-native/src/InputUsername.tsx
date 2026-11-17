import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Dropdown } from './Dropdown';
import { Tabs } from './Tabs';
import { TextField } from './TextField';
import { color, space } from './tokens';

export type UsernameTab = 'Unique' | 'Name';

/** Name editor of Settings > Account. With `uniqueName` the tabs switch between a claimed unique
 * name (the list of `names`) and a free name with its `#suffix`; without it only the free name shows.
 * Figma Input/Username: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=998-2339
 */
export function InputUsername({
  label,
  uniqueName,
  tab,
  onTabChange,
  uniqueTabLabel,
  nameTabLabel,
  names,
  picked,
  onPick,
  value,
  onChangeText,
  suffix,
  placeholder,
  maxLength,
  error,
  disabled = false,
}: {
  label: string;
  uniqueName: boolean;
  tab: UsernameTab;
  onTabChange: (tab: UsernameTab) => void;
  uniqueTabLabel: string;
  nameTabLabel: string;
  names: readonly string[];
  picked: string | null;
  onPick: (name: string) => void;
  value: string;
  onChangeText: (text: string) => void;
  /** `#1234` of an unclaimed name, in Input/Suffix. */
  suffix?: string;
  placeholder?: string;
  maxLength?: number;
  error?: string;
  disabled?: boolean;
}) {
  const showUnique = uniqueName && tab === 'Unique';
  return (
    <View style={styles.field}>
      <AppText variant="Input">{label}</AppText>
      <View style={styles.selector}>
        {uniqueName && (
          <Tabs
            items={[
              { key: 'Unique', label: uniqueTabLabel },
              { key: 'Name', label: nameTabLabel },
            ]}
            active={tab}
            onChange={onTabChange}
            gap={13}
          />
        )}
        {showUnique ? (
          <Dropdown
            label=""
            value={picked}
            options={names.map(name => ({
              value: name,
              label: name,
              icon: 'nameVerified',
            }))}
            onChange={onPick}
            disabled={disabled}
          />
        ) : (
          <TextField
            label=""
            value={value}
            onChangeText={onChangeText}
            placeholder={placeholder}
            maxLength={maxLength}
            showCounter={maxLength !== undefined}
            counterBelow
            error={error}
            disabled={disabled}
            autoCapitalize="none"
            suffix={
              suffix ? (
                <AppText variant="Input/Suffix" style={styles.suffix}>
                  {suffix}
                </AppText>
              ) : undefined
            }
          />
        )}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  field: { gap: space['20'] },
  selector: { gap: space.md },
  suffix: { color: color.text.placeholder },
});
