import { useEffect, useRef, useState } from 'react';
import { Animated, Easing, StyleSheet, View, ViewStyle } from 'react-native';
import { LinearFill } from './Gradient';
import { colors } from './tokens';
import { u } from './scale';

const WAVE_MS = 1000;
const PULSE_MS = 750;

const band = (color: string) => [
  { offset: 0, color, opacity: 0 },
  { offset: 0.484, color, opacity: 0.08 },
  { offset: 1, color, opacity: 0 },
];
const BAND = { dark: band('#FFFFFF'), light: band('#000000') };

/** Figma `Skeleton`: an 8% block (White on dark surfaces, Black on light ones). Wave sweeps a
 * half-width highlight across it in 1 s ease-out; Pulse fades it to 4% and back, 0.75 s each way. */
export function Skeleton({
  width,
  height,
  radius = 2,
  animation = 'wave',
  theme = 'dark',
  style,
}: {
  width: number | `${number}%`;
  height: number;
  radius?: number;
  animation?: 'wave' | 'pulse';
  theme?: 'dark' | 'light';
  style?: ViewStyle;
}) {
  const [boxWidth, setBoxWidth] = useState(0);
  const progress = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    const loop = Animated.loop(
      animation === 'wave'
        ? Animated.timing(progress, {
            toValue: 1,
            duration: WAVE_MS,
            easing: Easing.out(Easing.ease),
            useNativeDriver: true,
          })
        : Animated.sequence([
            Animated.timing(progress, {
              toValue: 1,
              duration: PULSE_MS,
              easing: Easing.inOut(Easing.ease),
              useNativeDriver: true,
            }),
            Animated.timing(progress, {
              toValue: 0,
              duration: PULSE_MS,
              easing: Easing.inOut(Easing.ease),
              useNativeDriver: true,
            }),
          ]),
    );
    loop.start();
    return () => loop.stop();
  }, [animation, progress]);

  const fill = theme === 'dark' ? colors.white8 : colors.black8;
  return (
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      onLayout={e => setBoxWidth(e.nativeEvent.layout.width)}
      style={[
        styles.box,
        { width, height: u(height), borderRadius: u(radius) },
        style,
      ]}
    >
      {animation === 'pulse' ? (
        <Animated.View
          style={[
            StyleSheet.absoluteFill,
            {
              backgroundColor: fill,
              opacity: progress.interpolate({
                inputRange: [0, 1],
                outputRange: [1, 0.5],
              }),
            },
          ]}
        />
      ) : (
        <>
          <View style={[StyleSheet.absoluteFill, { backgroundColor: fill }]} />
          <Animated.View
            style={[
              styles.wave,
              {
                width: boxWidth / 2,
                transform: [
                  {
                    translateX: progress.interpolate({
                      inputRange: [0, 1],
                      outputRange: [-boxWidth / 2, boxWidth],
                    }),
                  },
                ],
              },
            ]}
          >
            <LinearFill stops={BAND[theme]} angle={90} />
          </Animated.View>
        </>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  box: { overflow: 'hidden' },
  wave: { position: 'absolute', top: 0, bottom: 0, left: 0 },
});
