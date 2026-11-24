import { ScrollView, StyleSheet } from 'react-native';
import { layout, space, Swatch } from '@dcl/ui-native';
import { t } from '../i18n';

/** Figma `Presets`: the picker (ringed when the colour is custom) and the preset swatches. */
export function SwatchStrip({
  palette,
  value,
  custom,
  disabled = false,
  onSelect,
  onWheel,
}: {
  palette: string[];
  value: string;
  /** True when `value` is not one of the presets: the picker gets the ring. */
  custom: boolean;
  disabled?: boolean;
  onSelect: (color: string) => void;
  /** Omit it for the editor's own row, which has no picker. */
  onWheel?: () => void;
}) {
  const selected = (color: string) =>
    !custom && color.toLowerCase() === value.toLowerCase();
  return (
    <ScrollView
      removeClippedSubviews={false}
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={[styles.row, disabled && styles.disabled]}
    >
      {onWheel ? (
        <Swatch
          picker
          selected={custom}
          disabled={disabled}
          accessibilityLabel={t('BACKPACK_CUSTOM_COLOR')}
          onPress={onWheel}
        />
      ) : null}
      {palette.map(color => (
        <Swatch
          key={color}
          color={color}
          selected={selected(color)}
          disabled={disabled}
          onPress={() => onSelect(color)}
        />
      ))}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  row: {
    paddingHorizontal: layout.gutter,
    paddingVertical: space['2xs'],
    gap: space['30'],
    alignItems: 'center',
  },
  disabled: { opacity: 0.4 },
});
