import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { color, space } from './tokens';
import { u } from './scale';

/** A settings row: the label (and description) with its control. Switch / SwitchDescription put the
 * `control` on the right; Slider puts `value` beside the label and the control (a `Slider` with
 * `showLabel={false}`) below at full width.
 * Disabled dims the text; pass `disabled` to the control as well.
 * Figma Settings/Row: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=924-45
 */
export function SettingsRow({
  type = 'Switch',
  label,
  description,
  value,
  control,
  disabled = false,
}: {
  type?: 'Switch' | 'SwitchDescription' | 'Slider';
  label: string;
  description?: string;
  value?: string;
  control: ReactNode;
  disabled?: boolean;
}) {
  const text = disabled ? color.text.disabled : color.text.primary;
  if (type === 'Slider') {
    return (
      <View style={styles.slider}>
        <View style={styles.sliderLabel}>
          <AppText variant="Input" style={[styles.grow, { color: text }]}>
            {label}
          </AppText>
          {value !== undefined && (
            <AppText variant="Input/Value" style={{ color: text }}>
              {value}
            </AppText>
          )}
        </View>
        {control}
      </View>
    );
  }
  const withDescription = type === 'SwitchDescription';
  return (
    <View style={[styles.row, withDescription && styles.top]}>
      <View style={[styles.grow, withDescription && styles.texts]}>
        <AppText variant="Input" style={{ color: text }}>
          {label}
        </AppText>
        {withDescription && description ? (
          <AppText
            variant="Caption2"
            style={{
              color: disabled ? color.text.disabled : color.text.secondary,
            }}
          >
            {description}
          </AppText>
        ) : null}
      </View>
      {control}
    </View>
  );
}

const styles = StyleSheet.create({
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: space.xl,
    paddingVertical: space.xs,
  },
  top: { alignItems: 'flex-start' },
  grow: { flex: 1 },
  texts: { gap: space['10'], paddingBottom: space.xs },
  slider: { gap: space['20'], paddingTop: space.xs, paddingBottom: space.md },
  sliderLabel: {
    height: u(18),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
});
