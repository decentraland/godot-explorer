import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import {
  AppText,
  colors,
  fonts,
  SettingsRow,
  space,
  Switch,
  u,
} from '@dcl/ui-native';

const CAPTION_GAP = 22;

/** Lavender caps caption over a 1px divider that starts every group. */
export function SectionCaption({
  title,
  gap = CAPTION_GAP,
}: {
  title: string;
  /** Space under the divider in dp; link rows sit closer than switches and sliders. */
  gap?: number;
}) {
  return (
    <View style={{ marginBottom: u(gap) }}>
      <AppText variant="label" style={styles.captionText}>
        {title}
      </AppText>
      <View style={styles.divider} />
    </View>
  );
}

/** `Settings/Row` with a switch. The group's gap already spaces the rows as Figma does, so the row's
 * own vertical padding is taken back. */
export function ToggleRow({
  label,
  description,
  value,
  onChange,
  disabled = false,
}: {
  label: string;
  description?: string;
  value: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <View style={styles.toggleRow}>
      <SettingsRow
        type={description ? 'SwitchDescription' : 'Switch'}
        label={label}
        description={description}
        disabled={disabled}
        control={
          <Switch
            label={label}
            value={value}
            onChange={onChange}
            disabled={disabled}
          />
        }
      />
    </View>
  );
}

/** Vertical rhythm between the rows of one group. */
export function Group({
  children,
  gap = 30,
  after = 38,
}: {
  children: ReactNode;
  gap?: number;
  /** Space before the next caption. */
  after?: number;
}) {
  return (
    <View style={{ gap: u(gap), marginBottom: u(after) }}>{children}</View>
  );
}

const styles = StyleSheet.create({
  captionText: {
    color: colors.lavender,
    fontFamily: fonts.bold,
    fontSize: u(12),
    paddingBottom: u(4),
  },
  divider: { height: 1, backgroundColor: colors.lavender },
  toggleRow: { marginVertical: -space.xs },
});
