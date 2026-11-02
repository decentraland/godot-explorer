import { useEffect, useRef, useState } from 'react';
import { Animated, Easing, StyleSheet, View, ViewStyle } from 'react-native';
import { LinearFill } from './Gradient';
import { u } from './scale';

const SHIMMER_MS = 1200;

const SHEEN = [
  { offset: 0, color: '#FFFFFF', opacity: 0 },
  { offset: 0.5, color: '#FFFFFF', opacity: 0.12 },
  { offset: 1, color: '#FFFFFF', opacity: 0 },
];

/** Translucent placeholder block with a diagonal sheen sweeping across it. */
export function Skeleton({
  width,
  height,
  radius = 8,
  style,
}: {
  width: number | `${number}%`;
  height: number;
  radius?: number;
  style?: ViewStyle;
}) {
  const [boxWidth, setBoxWidth] = useState(0);
  const sweep = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    const loop = Animated.loop(
      Animated.timing(sweep, {
        toValue: 1,
        duration: SHIMMER_MS,
        easing: Easing.linear,
        useNativeDriver: true,
      }),
    );
    loop.start();
    return () => loop.stop();
  }, [sweep]);

  const translateX = sweep.interpolate({
    inputRange: [0, 1],
    outputRange: [-boxWidth, boxWidth],
  });

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
      <Animated.View
        style={[StyleSheet.absoluteFill, { transform: [{ translateX }] }]}
      >
        <LinearFill stops={SHEEN} angle={90} />
      </Animated.View>
    </View>
  );
}

const styles = StyleSheet.create({
  box: { backgroundColor: 'rgba(255,255,255,0.07)', overflow: 'hidden' },
});
