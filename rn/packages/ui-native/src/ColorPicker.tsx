import { ReactElement, useMemo, useRef, useState } from 'react';
import {
  Image,
  PanResponder,
  Pressable,
  ScrollView,
  StyleSheet,
  View,
} from 'react-native';
import { AppText } from './AppText';
import { hexToHsv, Hsv, hsvToHex } from './colorMath';
import { LinearFill } from './Gradient';
import { color, colors, elevation, minTouchTarget, radius, space } from './tokens';
import { u } from './scale';
import { t } from './i18n';

const HUE_STOPS = [0, 60, 120, 180, 240, 300, 360].map((hue, index) => ({
  offset: index / 6,
  color: hsvToHex({ h: hue, s: 1, v: 1 }),
}));

const SWATCH = u(40);
const RING_OUTSET = u(2.5);
const RING_WIDTH = u(1.5);

/** `Color-Swatch` Type=Picker: a blurred colour wheel react-native-svg cannot draw, exported as PNG. */
const pickerImage = require('../assets/color-picker.png');

/**
 * `Color-Swatch` (40dp): `color` for Type=Solid, `picker` for the wheel that opens the sliders.
 * Selected = 1.5dp `border/accent` ring 2.5dp outside; Pressed = swatch inset 2.5dp inside a Lavender ring.
 * Figma Color-Swatch: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=962-16569
 */
export function Swatch({
  color: fill,
  picker = false,
  selected,
  disabled,
  accessibilityLabel,
  onPress,
}: {
  color?: string;
  picker?: boolean;
  selected: boolean;
  disabled?: boolean;
  accessibilityLabel?: string;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel ?? fill}
      accessibilityState={{ selected, disabled }}
      disabled={disabled}
      onPress={onPress}
      hitSlop={Math.max(0, (minTouchTarget - SWATCH) / 2)}
      style={styles.swatch}
    >
      {({ pressed }) => (
        <>
          <View style={[styles.swatchFill, pressed && styles.swatchInset]}>
            {picker ? (
              <Image source={pickerImage} style={styles.swatchImage} />
            ) : (
              <View
                style={[styles.swatchImage, { backgroundColor: fill }]}
              />
            )}
          </View>
          {pressed ? (
            <View style={[styles.swatchRing, styles.swatchRingPressed]} />
          ) : selected ? (
            <View style={[styles.swatchRing, styles.swatchRingSelected]} />
          ) : null}
        </>
      )}
    </Pressable>
  );
}

export function SwatchRow({
  palette,
  value,
  onChange,
}: {
  palette: string[];
  value: string;
  onChange: (color: string) => void;
}) {
  return (
    <ScrollView
      removeClippedSubviews={false}
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.row}
    >
      {palette.map(preset => (
        <Swatch
          key={preset}
          color={preset}
          selected={preset.toLowerCase() === value.toLowerCase()}
          onPress={() => onChange(preset)}
        />
      ))}
    </ScrollView>
  );
}

function Slider({
  label,
  value,
  track,
  onChange,
}: {
  label: string;
  value: number;
  track: ReactTrack;
  onChange: (next: number) => void;
}) {
  const [width, setWidth] = useState(0);
  const widthRef = useRef(0);
  const responder = useMemo(() => {
    const move = (x: number) => {
      if (widthRef.current > 0) {
        onChange(Math.min(1, Math.max(0, x / widthRef.current)));
      }
    };
    return PanResponder.create({
      onStartShouldSetPanResponder: () => true,
      onMoveShouldSetPanResponder: () => true,
      onPanResponderTerminationRequest: () => false,
      onPanResponderGrant: event => move(event.nativeEvent.locationX),
      onPanResponderMove: event => move(event.nativeEvent.locationX),
    });
  }, [onChange]);
  return (
    <View style={styles.sliderBlock}>
      <AppText variant="Tags" style={styles.sliderLabel}>
        {label}
      </AppText>
      <View
        style={styles.track}
        onLayout={event => {
          widthRef.current = event.nativeEvent.layout.width;
          setWidth(event.nativeEvent.layout.width);
        }}
        {...responder.panHandlers}
      >
        {track}
        <View
          pointerEvents="none"
          style={[
            styles.thumb,
            { left: value * width - THUMB / 2 },
          ]}
        />
      </View>
    </View>
  );
}

type ReactTrack = ReactElement;
const THUMB = u(32);

/** Hue / saturation / brightness sliders (Figma "Hair color" panel).
 * Figma Control/ColorSlider: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=964-188
 */
export function HsvSliders({
  value,
  onChange,
}: {
  value: string;
  onChange: (hex: string) => void;
}) {
  const [hsv, setHsv] = useState<Hsv>(() => hexToHsv(value));
  const update = (patch: Partial<Hsv>) => {
    const next = { ...hsv, ...patch };
    setHsv(next);
    onChange(hsvToHex(next));
  };
  return (
    <View style={styles.sliders}>
      <Slider
        label={t('APP_COLOR_HUE')}
        value={hsv.h / 360}
        onChange={next => update({ h: next * 360 })}
        track={<LinearFill angle={90} stops={HUE_STOPS} />}
      />
      <Slider
        label={t('APP_COLOR_SATURATION')}
        value={hsv.s}
        onChange={next => update({ s: next })}
        track={
          <LinearFill
            angle={90}
            stops={[
              { offset: 0, color: hsvToHex({ h: hsv.h, s: 0, v: hsv.v }) },
              { offset: 1, color: hsvToHex({ h: hsv.h, s: 1, v: hsv.v }) },
            ]}
          />
        }
      />
      <Slider
        label={t('APP_COLOR_BRIGHTNESS')}
        value={hsv.v}
        onChange={next => update({ v: next })}
        track={
          <LinearFill
            angle={90}
            stops={[
              { offset: 0, color: '#000000' },
              { offset: 1, color: hsvToHex({ h: hsv.h, s: hsv.s, v: 1 }) },
            ]}
          />
        }
      />
    </View>
  );
}

const styles = StyleSheet.create({
  /** `Presets` row: swatches 30 px apart, the selected ring drawn into the gap. */
  row: { gap: space['30'], paddingVertical: RING_OUTSET },
  swatch: { width: SWATCH, height: SWATCH },
  swatchFill: { flex: 1, borderRadius: radius.pill, overflow: 'hidden' },
  swatchInset: { margin: RING_OUTSET },
  swatchImage: { width: '100%', height: '100%' },
  swatchRing: {
    position: 'absolute',
    borderRadius: radius.pill,
    borderWidth: RING_WIDTH,
  },
  swatchRingSelected: {
    top: -RING_OUTSET,
    left: -RING_OUTSET,
    right: -RING_OUTSET,
    bottom: -RING_OUTSET,
    borderColor: color.border.accent,
  },
  swatchRingPressed: {
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    borderColor: color.action.selected,
  },
  /** `Control/ColorSliderLabeled` stack: 56 px apart; label, 8 px, then the 64 px slider box. */
  sliders: { gap: space['56'] },
  sliderBlock: { gap: space.xs },
  sliderLabel: { color: color.text.primary },
  /** `Slider`: 48 px track centred in the 64 px box the thumb fills. */
  track: {
    height: u(24),
    marginVertical: space.xs,
    borderRadius: radius.sm,
    justifyContent: 'center',
  },
  /** No fill: the track under it shows the picked colour. */
  thumb: {
    position: 'absolute',
    top: -space.xs,
    width: THUMB,
    height: THUMB,
    borderRadius: radius.sm,
    borderWidth: space.xs,
    borderColor: colors.pearl,
    ...elevation.down,
  },
});
