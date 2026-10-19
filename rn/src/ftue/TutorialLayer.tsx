import { ReactNode, useEffect, useRef, useState } from 'react';
import {
  Animated,
  Easing,
  Image,
  Pressable,
  StyleSheet,
  Text,
  useWindowDimensions,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import Svg, { Path } from 'react-native-svg';
import { useNav } from '../app/NavProvider';
import { core } from '../core';
import { onCoreEvents } from '../core/events';
import { TutorialStep } from '../core/native';
import type { AvatarState } from '../core/native';
import { useCoreState } from '../core/useCoreState';
import { colors, Dialog, fonts, u } from '../ds';
import { t } from '../i18n';
import type { I18nKey } from '../i18n';
import { useChatOpen } from '../hud/hudSignals';
import {
  hudButtonRects,
  hudInsets,
  Insets,
  joystickCenter,
  joystickZone,
  Rect,
} from '../hud/layout';
import { closeMenu, openMenu, selectSection, useDrawerState } from '../menu';
import { railRect } from '../menu/DrawerShell';
import { useModalShowing } from '../modals/bridge';
import { FtueTag, Notch } from './FtueTag';
import { answerWelcome, skipTutorial, tutorialFact, useFtue } from './store';
import type { Run } from './tutorial';

const handDrag = require('../assets/ftue/hand_drag.png');
const pinchIn = require('../assets/ftue/pinch_in.png');
const pinchOut = require('../assets/ftue/pinch_out.png');

const RING = u(97);
const RING_MARGIN = u(11);
const HOLE_MARGIN = u(4);
const GAP = u(8);
const PINCH_SWAP_MS = 900;

const STEP_TEXT: Record<TutorialStep, I18nKey> = {
  [TutorialStep.Move]: 'FTUE_TUTORIAL_MOVE',
  [TutorialStep.Camera]: 'FTUE_TUTORIAL_CAMERA',
  [TutorialStep.Pinch]: 'FTUE_TUTORIAL_PINCH',
  [TutorialStep.Chat]: 'FTUE_TUTORIAL_CHAT',
  [TutorialStep.Menu]: 'FTUE_TUTORIAL_MENU',
  [TutorialStep.Social]: 'FTUE_TUTORIAL_SOCIAL',
  [TutorialStep.Backpack]: 'FTUE_TUTORIAL_BACKPACK',
  [TutorialStep.Equip]: 'FTUE_TUTORIAL_EQUIP',
};

const grow = (r: Rect, by: number): Rect => ({
  left: r.left - by,
  top: r.top - by,
  width: r.width + 2 * by,
  height: r.height + 2 * by,
});

const centre = (r: Rect) => ({
  x: r.left + r.width / 2,
  y: r.top + r.height / 2,
});

const box = (left: number, top: number, width: number, height: number) => ({
  position: 'absolute' as const,
  left,
  top,
  width: Math.max(0, width),
  height: Math.max(0, height),
});

/** Swallows every touch outside `hole`; the hole reaches whatever is under it. */
function Blocker({ hole }: { hole?: Rect }) {
  const { width, height } = useWindowDimensions();
  const swallow = { onStartShouldSetResponder: () => true };
  if (!hole) {
    return <View style={StyleSheet.absoluteFill} {...swallow} />;
  }
  const right = hole.left + hole.width;
  const bottom = hole.top + hole.height;
  return (
    <>
      <View style={box(0, 0, width, hole.top)} {...swallow} />
      <View style={box(0, bottom, width, height - bottom)} {...swallow} />
      <View style={box(0, hole.top, hole.left, hole.height)} {...swallow} />
      <View
        style={box(right, hole.top, width - right, hole.height)}
        {...swallow}
      />
    </>
  );
}

/** The scene dimmed except for a rounded window over `hole`. */
function Dim({ hole, radius }: { hole: Rect; radius: number }) {
  const { width, height } = useWindowDimensions();
  const { left: x, top: y, width: w, height: h } = hole;
  const r = Math.min(radius, w / 2, h / 2);
  const d =
    `M0 0H${width}V${height}H0Z ` +
    `M${x + r} ${y}H${x + w - r}A${r} ${r} 0 0 1 ${x + w} ${y + r}` +
    `V${y + h - r}A${r} ${r} 0 0 1 ${x + w - r} ${y + h}` +
    `H${x + r}A${r} ${r} 0 0 1 ${x} ${y + h - r}` +
    `V${y + r}A${r} ${r} 0 0 1 ${x + r} ${y}Z`;
  return (
    <Svg style={StyleSheet.absoluteFill} pointerEvents="none">
      <Path d={d} fill="rgba(0,0,0,0.6)" fillRule="evenodd" />
    </Svg>
  );
}

function Ring({
  x,
  y,
  size,
  animate,
}: {
  x: number;
  y: number;
  size: number;
  animate: boolean;
}) {
  const pulse = useRef(new Animated.Value(0)).current;
  useEffect(() => {
    if (!animate) {
      return undefined;
    }
    const loop = Animated.loop(
      Animated.timing(pulse, {
        toValue: 1,
        duration: 1200,
        easing: Easing.out(Easing.quad),
        useNativeDriver: true,
      }),
    );
    loop.start();
    return () => loop.stop();
  }, [animate, pulse]);
  const scale = pulse.interpolate({
    inputRange: [0, 1],
    outputRange: [1, 1.15],
  });
  const opacity = pulse.interpolate({
    inputRange: [0, 1],
    outputRange: [1, 0.3],
  });
  return (
    <Animated.View
      pointerEvents="none"
      style={[
        styles.ring,
        {
          left: x - size / 2,
          top: y - size / 2,
          width: size,
          height: size,
          borderRadius: size / 2,
          opacity,
          transform: [{ scale }],
        },
      ]}
    />
  );
}

type Place = 'above' | 'below' | 'right' | 'center';

const NOTCH: Record<Place, Notch | undefined> = {
  above: 'down',
  below: 'up',
  right: 'left',
  center: undefined,
};

/** The step's instruction next to (x, y): its edge for above/below/right, its centre otherwise. */
export function Tooltip({
  text,
  x,
  y,
  place,
}: {
  text: I18nKey;
  x: number;
  y: number;
  place: Place;
}) {
  const transform =
    place === 'right'
      ? [{ translateY: '-50%' as const }]
      : [
          { translateX: '-50%' as const },
          ...(place === 'center'
            ? [{ translateY: '-50%' as const }]
            : place === 'above'
            ? [{ translateY: '-100%' as const }]
            : []),
        ];
  return (
    <View
      pointerEvents="none"
      style={[styles.abs, { left: x, top: y, transform }]}
    >
      <FtueTag label={t(text)} notch={NOTCH[place]} small />
    </View>
  );
}

function Gesture({ pinch, animate }: { pinch: boolean; animate: boolean }) {
  const { width, height } = useWindowDimensions();
  const [inward, setInward] = useState(false);
  useEffect(() => {
    if (!pinch || !animate) {
      return undefined;
    }
    const timer = setInterval(() => setInward(v => !v), PINCH_SWAP_MS);
    return () => clearInterval(timer);
  }, [pinch, animate]);
  const source = pinch ? (inward ? pinchIn : pinchOut) : handDrag;
  const size = pinch
    ? inward
      ? { width: u(267 / 2), height: u(280 / 2) }
      : { width: u(297 / 2), height: u(366 / 2) }
    : { width: u(236 / 2), height: u(364 / 2) };
  return (
    <Image
      source={source}
      resizeMode="contain"
      style={[
        styles.abs,
        size,
        {
          left: width * 0.66 - size.width / 2,
          top: height * 0.62 - size.height / 2,
        },
      ]}
    />
  );
}

function SkipButton({ insets }: { insets: Insets }) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('FTUE_TUTORIAL_SKIP')}
      onPress={skipTutorial}
      style={[styles.skip, { top: insets.top, right: insets.right }]}
    >
      <Text style={styles.skipText}>{t('FTUE_TUTORIAL_SKIP')}</Text>
    </Pressable>
  );
}

/** A HUD button or a rail entry, lit through the dim and the only thing that takes touches. */
function Highlight({
  target,
  text,
  place,
  window,
  windowRadius,
  animate,
}: {
  target: Rect;
  text: I18nKey;
  place: Place;
  window: Rect;
  windowRadius: number;
  animate: boolean;
}) {
  const c = centre(target);
  const tip =
    place === 'right'
      ? { x: target.left + target.width + RING_MARGIN + GAP, y: c.y }
      : { x: c.x, y: target.top + target.height + RING_MARGIN + GAP };
  return (
    <>
      <Dim hole={window} radius={windowRadius} />
      <Blocker hole={target} />
      <Ring
        x={c.x}
        y={c.y}
        size={Math.max(target.width, target.height) + 2 * RING_MARGIN}
        animate={animate}
      />
      <Tooltip text={text} x={tip.x} y={tip.y} place={place} />
    </>
  );
}

/** What one step draws over the world. */
export function StepOverlay({
  run,
  insets,
  animate = true,
}: {
  run: Run;
  insets: Insets;
  animate?: boolean;
}) {
  const { width, height } = useWindowDimensions();
  const buttons = hudButtonRects(insets);
  const idle = !run.engaged && !run.next;
  const text = STEP_TEXT[run.step];
  let body: ReactNode = null;
  switch (run.step) {
    case TutorialStep.Move: {
      const c = joystickCenter(insets);
      // Only the joystick takes touches, so a drag elsewhere cannot turn the camera.
      body = (
        <>
          <Blocker hole={joystickZone(height)} />
          {idle && (
            <>
              <Ring x={c.x} y={c.y} size={RING} animate={animate} />
              <Tooltip
                text={text}
                x={c.x}
                y={c.y - RING / 2 - GAP}
                place="above"
              />
            </>
          )}
        </>
      );
      break;
    }
    case TutorialStep.Camera:
    case TutorialStep.Pinch:
      body = idle && (
        <>
          <Tooltip
            text={text}
            x={width * 0.5}
            y={height * 0.24}
            place="center"
          />
          <Gesture pinch={run.step === TutorialStep.Pinch} animate={animate} />
        </>
      );
      break;
    case TutorialStep.Chat:
    case TutorialStep.Menu: {
      const target =
        run.step === TutorialStep.Chat ? buttons.chat : buttons.menu;
      body = idle ? (
        <Highlight
          target={target}
          text={text}
          place="below"
          window={grow(target, HOLE_MARGIN)}
          windowRadius={target.width}
          animate={animate}
        />
      ) : (
        run.guardUntil !== undefined && <Blocker />
      );
      break;
    }
    case TutorialStep.Social:
    case TutorialStep.Backpack:
      body = idle && (
        <Highlight
          target={railRect(
            run.step === TutorialStep.Social ? 'social' : 'backpack',
          )}
          text={text}
          place="right"
          window={railRect()}
          windowRadius={u(12)}
          animate={animate}
        />
      );
      break;
    case TutorialStep.Equip:
      break;
  }
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="box-none">
      {body}
      {run.step !== TutorialStep.Equip && <SkipButton insets={insets} />}
    </View>
  );
}

/** The Backpack step's hint, drawn by the Backpack itself (it covers the world). */
export function EquipHint() {
  const { run } = useFtue();
  const { width } = useWindowDimensions();
  if (run?.step !== TutorialStep.Equip || run.engaged) {
    return null;
  }
  return (
    <Tooltip
      text="FTUE_TUTORIAL_EQUIP"
      x={width / 2}
      y={u(64)}
      place="center"
    />
  );
}

function sameOutfit(a: AvatarState | undefined, b: AvatarState) {
  return (
    a !== undefined &&
    a.bodyShape === b.bodyShape &&
    b.wearables.every(urn => a.wearables.includes(urn))
  );
}

/** Feeds the run what the player does: the engine's reports through the core, and the shell's
 * own chat, menu and Backpack. */
function useTutorialFacts(run: Run | null) {
  const active = run !== null;
  const step = run?.step;
  const settled = run !== null && !run.engaged && !run.next;
  const inWorld = useCoreState().tag === 'InWorld';
  const chatOpen = useChatOpen();
  const drawer = useDrawerState();
  const routeName = useNav().route.name;
  const onWorld = routeName === 'world';
  const outfit = useRef<AvatarState | undefined>(undefined);
  const wasInWorld = useRef(inWorld);

  useEffect(
    () =>
      active
        ? onCoreEvents({
            hud: hud => tutorialFact({ type: 'hud', hud, now: Date.now() }),
          })
        : undefined,
    [active],
  );
  useEffect(() => {
    if (active) {
      tutorialFact({ type: 'chat', open: chatOpen, now: Date.now() });
    }
  }, [active, chatOpen]);
  useEffect(() => {
    if (active) {
      tutorialFact({
        type: 'menu',
        open: drawer.open,
        social: drawer.section === 'social',
        now: Date.now(),
      });
    }
  }, [active, drawer.open, drawer.section]);
  useEffect(() => {
    if (active) {
      tutorialFact({ type: 'backpack', open: routeName === 'backpack' });
    }
  }, [active, routeName]);
  useEffect(() => {
    if (step !== TutorialStep.Equip) {
      return undefined;
    }
    try {
      outfit.current = core().avatarState();
    } catch {
      outfit.current = undefined;
    }
    return onCoreEvents({
      avatarChanged: avatar => {
        if (!sameOutfit(outfit.current, avatar)) {
          tutorialFact({ type: 'equipped' });
        }
      },
    });
  }, [step]);
  useEffect(() => {
    if (active && inWorld && !wasInWorld.current) {
      tutorialFact({ type: 'reenter' });
    }
    wasInWorld.current = inWorld;
  }, [active, inWorld]);
  // Puts the menu as the step needs it, when the player left it otherwise.
  useEffect(() => {
    if (!settled || !onWorld) {
      return;
    }
    const wantsMenu =
      step === TutorialStep.Social || step === TutorialStep.Backpack;
    if (wantsMenu && !drawer.open) {
      openMenu();
    } else if (!wantsMenu && drawer.open) {
      closeMenu();
    }
  }, [settled, onWorld, step, drawer.open]);
  const socialEngaged = step === TutorialStep.Social && run?.engaged === true;
  useEffect(() => {
    // New players have no friends yet; Nearby is where they can add some.
    if (socialEngaged) {
      selectSection('social', 'nearby');
    }
  }, [socialEngaged]);
}

/** Offers the tutorial; `onAnswer(true)` is START. */
export function WelcomeCard({
  visible,
  onAnswer,
}: {
  visible: boolean;
  onAnswer: (start: boolean) => void;
}) {
  return (
    <Dialog
      visible={visible}
      rounded
      title={t('FTUE_TUTORIAL_WELCOME_TITLE')}
      body={t('FTUE_TUTORIAL_WELCOME_BODY')}
      onRequestClose={() => {}}
      buttons={[
        {
          label: t('FTUE_TUTORIAL_WELCOME_SKIP'),
          tone: 'secondary',
          onPress: () => onAnswer(false),
        },
        {
          label: t('FTUE_TUTORIAL_WELCOME_START'),
          onPress: () => onAnswer(true),
        },
      ]}
    />
  );
}

/** The welcome card and the tutorial's steps, over the world and its menu. */
export function TutorialLayer({ animate = true }: { animate?: boolean }) {
  const { welcome, run } = useFtue();
  const insets = hudInsets(useSafeAreaInsets());
  const inWorld = useCoreState().tag === 'InWorld';
  const modal = useModalShowing();
  const onWorld = useNav().route.name === 'world';
  useTutorialFacts(run);
  return (
    <>
      <WelcomeCard
        visible={welcome && inWorld && !modal}
        onAnswer={answerWelcome}
      />
      {run && inWorld && !modal && onWorld && (
        <StepOverlay run={run} insets={insets} animate={animate} />
      )}
    </>
  );
}

const styles = StyleSheet.create({
  abs: { position: 'absolute' },
  ring: {
    position: 'absolute',
    borderWidth: u(1.5),
    borderColor: colors.lavender,
  },
  skip: {
    position: 'absolute',
    minHeight: u(22),
    justifyContent: 'center',
    paddingHorizontal: u(9),
    paddingVertical: u(5),
    borderRadius: u(4),
    backgroundColor: 'rgba(0,0,0,0.4)',
  },
  skipText: { fontFamily: fonts.bold, fontSize: u(9), color: colors.snow },
});
