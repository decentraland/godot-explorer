import { useEffect, useRef } from 'react';
import { AccessibilityInfo, Animated, Easing, StyleSheet } from 'react-native';
import Svg, { Circle } from 'react-native-svg';
import { colors } from './tokens';
import { u } from './scale';

/** Figma `Loader/Spinner` (150×150): a Black 40% ring with a Ruby arc that rotates 360° every
 * second, linear and infinite; slower when the OS asks for reduced motion. `size` is in dp; the
 * ring (radius 53, stroke 16) and the 159° arc keep the drawn node's ratios inside the box. */
export function Spinner({
  size = 75,
  accessibilityLabel,
}: {
  size?: number;
  accessibilityLabel?: string;
}) {
  const turn = useRef(new Animated.Value(0)).current;
  useEffect(() => {
    let loop: Animated.CompositeAnimation | undefined;
    let cancelled = false;
    AccessibilityInfo.isReduceMotionEnabled().then(reduced => {
      if (cancelled) {
        return;
      }
      loop = Animated.loop(
        Animated.timing(turn, {
          toValue: 1,
          duration: reduced ? 3000 : 1000,
          easing: Easing.linear,
          useNativeDriver: true,
        }),
      );
      loop.start();
    });
    return () => {
      cancelled = true;
      loop?.stop();
    };
  }, [turn]);
  const px = u(size);
  const stroke = (px * 16) / 150;
  const r = (px * 53) / 150;
  const circumference = 2 * Math.PI * r;
  const rotate = turn.interpolate({
    inputRange: [0, 1],
    outputRange: ['0deg', '360deg'],
  });
  return (
    <Animated.View
      accessibilityRole="progressbar"
      accessibilityLabel={accessibilityLabel}
      style={[styles.box, { width: px, height: px, transform: [{ rotate }] }]}
    >
      <Svg width={px} height={px} viewBox={`0 0 ${px} ${px}`}>
        <Circle
          cx={px / 2}
          cy={px / 2}
          r={r}
          stroke={colors.black40}
          strokeWidth={stroke}
          fill="none"
        />
        <Circle
          cx={px / 2}
          cy={px / 2}
          r={r}
          stroke={colors.ruby}
          strokeWidth={stroke}
          fill="none"
          strokeDasharray={`${circumference * 0.442} ${circumference}`}
          rotation={135}
          origin={`${px / 2}, ${px / 2}`}
        />
      </Svg>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  box: { alignItems: 'center', justifyContent: 'center' },
});
