import { useEffect, useRef } from 'react';
import {
  Animated,
  Easing,
  Pressable,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import Svg, { Path } from 'react-native-svg';
import { colors, fonts, Icon, u } from '../ds';
import { t } from '../i18n';
import type { I18nKey } from '../i18n';
import type { CoachStep } from './coachMarks';
import {
  GAMEPAD_LABEL,
  HUD_FRAME,
  HUD_MARGIN_FLOOR,
  Insets,
  JOYSTICK_BASE,
} from './layout';

type Mark = {
  key: I18nKey;
  x: number;
  y: number;
  notch: 'up' | 'down' | 'left';
  anchor: 'left' | 'right';
};

/** Centres in Figma px (1600x720, final HUD layout of frame 275). */
const CONTROL_MARKS: Mark[] = [
  { key: 'COACH_MENU', x: 150, y: 166, notch: 'up', anchor: 'left' },
  { key: 'COACH_CHAT', x: 256, y: 142, notch: 'up', anchor: 'left' },
  {
    key: 'COACH_MOVEMENT',
    x: JOYSTICK_BASE.x + JOYSTICK_BASE.width / 2 - 10,
    y: 392,
    notch: 'down',
    anchor: 'left',
  },
  { key: 'COACH_EMOTES', x: 232, y: 633, notch: 'left', anchor: 'left' },
  {
    key: 'COACH_CONTROLS',
    x: GAMEPAD_LABEL.x,
    y: GAMEPAD_LABEL.y,
    notch: 'down',
    anchor: 'right',
  },
];

/** Figma px -> dp offsets inside the HUD insets, so marks follow the same floor as the real buttons. */
function place(mark: Mark, insets: Insets) {
  const top = insets.top + u((mark.y - HUD_MARGIN_FLOOR.top) / 2);
  const centred = mark.anchor === 'left' ? '-50%' : '50%';
  const transform = [
    { translateY: '-50%' as const },
    ...(mark.notch === 'left' ? [] : [{ translateX: centred as `${number}%` }]),
  ];
  return mark.anchor === 'left'
    ? {
        top,
        left: insets.left + u((mark.x - HUD_MARGIN_FLOOR.left) / 2),
        transform,
      }
    : {
        top,
        right:
          insets.right +
          u((HUD_FRAME.width - HUD_MARGIN_FLOOR.right - mark.x) / 2),
        transform,
      };
}

function Pill({ label, notch }: { label: string; notch?: Mark['notch'] }) {
  return (
    <View style={styles.pill}>
      <Text style={styles.pillText}>{label}</Text>
      {notch && <View style={[styles.notch, styles[notch]]} />}
    </View>
  );
}

function CameraHint({ animate }: { animate: boolean }) {
  const swing = useRef(new Animated.Value(0)).current;
  useEffect(() => {
    if (!animate) {
      return undefined;
    }
    const loop = Animated.loop(
      Animated.sequence([
        Animated.timing(swing, {
          toValue: 1,
          duration: 700,
          easing: Easing.inOut(Easing.quad),
          useNativeDriver: true,
        }),
        Animated.timing(swing, {
          toValue: 0,
          duration: 700,
          easing: Easing.inOut(Easing.quad),
          useNativeDriver: true,
        }),
      ]),
    );
    loop.start();
    return () => loop.stop();
  }, [animate, swing]);
  const rotate = swing.interpolate({
    inputRange: [0, 1],
    outputRange: ['-12deg', '12deg'],
  });
  return (
    <View style={styles.hint} pointerEvents="none">
      <Svg
        width={u(24)}
        height={u(60)}
        viewBox="0 0 24 60"
        style={styles.arrows}
      >
        <Path
          d="M18 6 C4 16 4 44 18 54 M18 6 L11 8 M18 6 L19 13 M18 54 L11 52 M18 54 L19 47"
          stroke={colors.lavender}
          strokeWidth={1.6}
          strokeLinecap="round"
          fill="none"
        />
      </Svg>
      <Animated.View style={{ transform: [{ rotate }] }}>
        <Icon name="hand" size={96} color={colors.lavender} strokeWidth={1.4} />
      </Animated.View>
    </View>
  );
}

/** First-run tour over the HUD: tap anywhere to move on. Step 1 labels the controls, step 2 the camera gesture. */
export function CoachMarks({
  step,
  insets,
  onNext,
  animate = true,
}: {
  step: CoachStep;
  insets: Insets;
  onNext: () => void;
  animate?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('COACH_NEXT')}
      onPress={onNext}
      style={styles.scrim}
    >
      {step === 'controls' ? (
        CONTROL_MARKS.map(mark => (
          <View key={mark.key} style={[styles.mark, place(mark, insets)]}>
            <Pill label={t(mark.key)} notch={mark.notch} />
          </View>
        ))
      ) : (
        <>
          <View style={styles.center}>
            <Pill label={t('COACH_SWITCH_CAMERA')} />
          </View>
          <CameraHint animate={animate} />
        </>
      )}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  scrim: { ...StyleSheet.absoluteFill, backgroundColor: 'rgba(30,0,12,0.55)' },
  mark: { position: 'absolute' },
  center: { position: 'absolute', top: '46%', alignSelf: 'center' },
  pill: {
    paddingHorizontal: u(7),
    paddingVertical: u(4.5),
    borderRadius: u(6),
    backgroundColor: 'rgba(53,8,84,0.85)',
  },
  pillText: { fontFamily: fonts.medium, fontSize: u(11.3), color: colors.snow },
  notch: {
    position: 'absolute',
    width: 0,
    height: 0,
    borderStyle: 'solid',
    borderColor: 'transparent',
  },
  up: {
    top: -u(4),
    left: '50%',
    marginLeft: -u(4),
    borderLeftWidth: u(4),
    borderRightWidth: u(4),
    borderBottomWidth: u(4),
    borderBottomColor: 'rgba(53,8,84,0.85)',
  },
  down: {
    bottom: -u(4),
    left: '50%',
    marginLeft: -u(4),
    borderLeftWidth: u(4),
    borderRightWidth: u(4),
    borderTopWidth: u(4),
    borderTopColor: 'rgba(53,8,84,0.85)',
  },
  left: {
    left: -u(4),
    top: '50%',
    marginTop: -u(4),
    borderTopWidth: u(4),
    borderBottomWidth: u(4),
    borderRightWidth: u(4),
    borderRightColor: 'rgba(53,8,84,0.85)',
  },
  hint: {
    position: 'absolute',
    top: '22%',
    right: '12%',
    flexDirection: 'row',
    alignItems: 'center',
  },
  arrows: { marginRight: u(4) },
});
