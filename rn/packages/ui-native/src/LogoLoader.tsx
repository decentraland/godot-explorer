import { useEffect, useRef } from 'react';
import { AccessibilityInfo, Animated, Easing } from 'react-native';
import { Icon } from './Icon';
import { motion } from './tokens';

/** Figma `Loader/Logo` (154×154): the Decentraland logo of full-screen loading, drawn from the
 * `nameLogo` vector so it stays sharp, with a subtle pulse (`motion.duration.slow` each way);
 * still when the OS asks for reduced motion.
 * Figma Loader/Logo: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=562-8818
 */
export function LogoLoader({
  size = 77,
  accessibilityLabel,
}: {
  size?: number;
  accessibilityLabel?: string;
}) {
  const pulse = useRef(new Animated.Value(0)).current;
  useEffect(() => {
    let loop: Animated.CompositeAnimation | undefined;
    let cancelled = false;
    AccessibilityInfo.isReduceMotionEnabled().then(reduced => {
      if (cancelled || reduced) {
        return;
      }
      const step = (toValue: number) =>
        Animated.timing(pulse, {
          toValue,
          duration: motion.duration.slow,
          easing: Easing.inOut(Easing.ease),
          useNativeDriver: true,
        });
      loop = Animated.loop(Animated.sequence([step(1), step(0)]));
      loop.start();
    });
    return () => {
      cancelled = true;
      loop?.stop();
    };
  }, [pulse]);
  const scale = pulse.interpolate({
    inputRange: [0, 1],
    outputRange: [1, 1.05],
  });
  return (
    <Animated.View
      accessibilityRole="progressbar"
      accessibilityLabel={accessibilityLabel}
      style={{ transform: [{ scale }] }}
    >
      <Icon name="nameLogo" size={size} />
    </Animated.View>
  );
}
