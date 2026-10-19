import { useEffect, useRef, useState } from 'react';
import { Animated, Pressable, StyleSheet, Text } from 'react-native';
import { ControlsFtueElement } from '../core/native';
import { colors, fonts, u } from '../ds';
import { t } from '../i18n';
import type { I18nKey } from '../i18n';
import { hudPoint, Insets, JOYSTICK_BASE } from '../hud/layout';
import { FtueTag, Notch } from './FtueTag';

export const FADE_MS = 200;
export const STAGGER_MS = 150;

type Mark = { key: I18nKey; x: number; y: number; notch: Notch };

/** Centres in Figma px (1600x720, final HUD layout of frame 275). */
const MARKS: Record<ControlsFtueElement, Mark> = {
  [ControlsFtueElement.Menu]: {
    key: 'CONTROLS_FTUE_MENU',
    x: 150,
    y: 166,
    notch: 'up',
  },
  [ControlsFtueElement.Chat]: {
    key: 'CONTROLS_FTUE_CHAT',
    x: 256,
    y: 142,
    notch: 'up',
  },
  [ControlsFtueElement.Movement]: {
    key: 'CONTROLS_FTUE_MOVEMENT',
    x: JOYSTICK_BASE.x + JOYSTICK_BASE.width / 2 - 10,
    y: 392,
    notch: 'down',
  },
  [ControlsFtueElement.Emotes]: {
    key: 'CONTROLS_FTUE_EMOTES',
    x: 232,
    y: 633,
    notch: 'left',
  },
};

/** Milliseconds from the overlay appearing to the tap prompt: the staggered entrance, then the lock. */
export const unlockAfterMs = (count: number, lockSeconds: number) =>
  STAGGER_MS * count + FADE_MS + lockSeconds * 1000;

function place(mark: Mark, insets: Insets) {
  const { x, y } = hudPoint(mark.x, mark.y, insets);
  return {
    top: y,
    left: x,
    transform: [
      { translateY: '-50%' as const },
      ...(mark.notch === 'left' ? [] : [{ translateX: '-50%' as const }]),
    ],
  };
}

function useFade(delayMs: number, animate: boolean, on = true) {
  const value = useRef(new Animated.Value(animate ? 0 : 1)).current;
  useEffect(() => {
    if (!animate || !on) {
      return undefined;
    }
    const fade = Animated.timing(value, {
      toValue: 1,
      duration: FADE_MS,
      delay: delayMs,
      useNativeDriver: true,
    });
    fade.start();
    return () => fade.stop();
  }, [animate, delayMs, on, value]);
  return value;
}

function Label({
  mark,
  insets,
  delayMs,
  animate,
}: {
  mark: Mark;
  insets: Insets;
  delayMs: number;
  animate: boolean;
}) {
  const opacity = useFade(delayMs, animate);
  return (
    <Animated.View style={[styles.mark, place(mark, insets), { opacity }]}>
      <FtueTag label={t(mark.key)} notch={mark.notch} />
    </Animated.View>
  );
}

/**
 * Dims the scene and labels the HUD controls one after another (#3014). Every tap is swallowed
 * until the lock is over; then "Tap the screen to start exploring" shows and a tap closes it.
 */
export function ControlsOverlay({
  elements,
  insets,
  lockSeconds,
  onDismiss,
  animate = true,
}: {
  elements: ControlsFtueElement[];
  insets: Insets;
  lockSeconds: number;
  /** With the seconds it was on screen. */
  onDismiss: (secondsShown: number) => void;
  animate?: boolean;
}) {
  const shownAt = useRef(Date.now()).current;
  const [unlocked, setUnlocked] = useState(false);
  const dim = useFade(0, animate);
  const prompt = useFade(0, animate, unlocked);

  useEffect(() => {
    const timer = setTimeout(
      () => setUnlocked(true),
      unlockAfterMs(elements.length, lockSeconds),
    );
    return () => clearTimeout(timer);
  }, [elements.length, lockSeconds]);

  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('CONTROLS_FTUE_TAP_TO_START')}
      onPress={() => unlocked && onDismiss((Date.now() - shownAt) / 1000)}
      style={StyleSheet.absoluteFill}
    >
      <Animated.View style={[styles.dim, { opacity: dim }]} />
      {elements.map((element, i) => (
        <Label
          key={element}
          mark={MARKS[element]}
          insets={insets}
          delayMs={STAGGER_MS * (i + 1)}
          animate={animate}
        />
      ))}
      {unlocked && (
        <Animated.View style={[styles.prompt, { opacity: prompt }]}>
          <Text style={styles.promptText}>
            {t('CONTROLS_FTUE_TAP_TO_START')}
          </Text>
        </Animated.View>
      )}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  dim: { ...StyleSheet.absoluteFill, backgroundColor: 'rgba(0,0,0,0.6)' },
  mark: { position: 'absolute' },
  prompt: { position: 'absolute', top: '31%', left: 0, right: 0 },
  promptText: {
    textAlign: 'center',
    fontFamily: fonts.medium,
    fontSize: u(14),
    color: colors.snow,
  },
});
