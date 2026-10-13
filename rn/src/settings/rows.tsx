import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { AppText, colors, fonts, Switch, u } from '../ds';

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

/** Label (plus optional description) on the left, switch on the right; disabled dims all of it. */
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
      <View style={[styles.toggleText, disabled && styles.dim]}>
        <View style={styles.labelBox}>
          <AppText variant="body" style={styles.rowLabel}>
            {label}
          </AppText>
        </View>
        {description && (
          <AppText variant="caption" style={styles.description}>
            {description}
          </AppText>
        )}
      </View>
      <Switch
        label={label}
        value={value}
        onChange={onChange}
        disabled={disabled}
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
  toggleRow: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    minHeight: u(32),
  },
  labelBox: { minHeight: u(32), justifyContent: 'center' },
  toggleText: { flex: 1, paddingRight: u(20) },
  rowLabel: { fontFamily: fonts.medium, fontSize: u(15), color: colors.snow },
  description: {
    fontFamily: fonts.regular,
    fontSize: u(11),
    color: colors.white70,
  },
  dim: { opacity: 0.35 },
});
