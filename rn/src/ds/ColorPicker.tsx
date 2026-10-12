import { ReactElement, useMemo, useRef, useState } from 'react';
import {
  PanResponder,
  Pressable,
  ScrollView,
  StyleSheet,
  View,
} from 'react-native';
import { AppText } from './AppText';
import { hexToHsv, Hsv, hsvToHex } from './colorMath';
import { LinearFill } from './Gradient';
import { colors } from './tokens';
import { u } from './scale';

const HUE_STOPS = [0, 60, 120, 180, 240, 300, 360].map((hue, index) => ({
  offset: index / 6,
  color: hsvToHex({ h: hue, s: 1, v: 1 }),
}));

export function Swatch({
  color,
  selected,
  onPress,
}: {
  color: string;
  selected: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={color}
      accessibilityState={{ selected }}
      onPress={onPress}
      style={[styles.swatchRing, selected && styles.swatchRingOn]}
    >
      <View style={[styles.swatch, { backgroundColor: color }]} />
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
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.row}
    >
      {palette.map(color => (
        <Swatch
          key={color}
          color={color}
          selected={color.toLowerCase() === value.toLowerCase()}
          onPress={() => onChange(color)}
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
  thumbColor,
}: {
  label: string;
  value: number;
  track: ReactTrack;
  onChange: (next: number) => void;
  thumbColor: string;
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
      <AppText variant="label">{label}</AppText>
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
            { left: value * width - THUMB / 2, backgroundColor: thumbColor },
          ]}
        />
      </View>
    </View>
  );
}

type ReactTrack = ReactElement;
const THUMB = u(32);

/** Hue / saturation / brightness sliders (Figma "Hair color" panel). */
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
  const hueOnly = hsvToHex({ h: hsv.h, s: 1, v: 1 });
  return (
    <View>
      <Slider
        label="Color"
        value={hsv.h / 360}
        thumbColor={hueOnly}
        onChange={next => update({ h: next * 360 })}
        track={<LinearFill angle={90} stops={HUE_STOPS} />}
      />
      <Slider
        label="Saturation"
        value={hsv.s}
        thumbColor={hsvToHex({ h: hsv.h, s: hsv.s, v: 1 })}
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
        label="Brightness"
        value={hsv.v}
        thumbColor={hsvToHex({ h: 0, s: 0, v: hsv.v })}
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
  row: { gap: u(10), paddingVertical: u(4) },
  swatchRing: {
    width: u(44),
    height: u(44),
    borderRadius: u(22),
    borderWidth: 2,
    borderColor: 'transparent',
    alignItems: 'center',
    justifyContent: 'center',
  },
  swatchRingOn: { borderColor: colors.ruby },
  swatch: {
    width: u(38),
    height: u(38),
    borderRadius: u(19),
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.45)',
  },
  sliderBlock: { marginTop: u(14) },
  track: {
    height: u(24),
    marginTop: u(8),
    borderRadius: u(3),
    justifyContent: 'center',
  },
  thumb: {
    position: 'absolute',
    top: u(-4),
    width: THUMB,
    height: u(32),
    borderRadius: u(3),
    borderWidth: 2,
    borderColor: colors.snow,
  },
});
