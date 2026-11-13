import { ReactNode, useEffect, useRef } from 'react';
import {
  Animated,
  Easing,
  Modal,
  Pressable,
  StyleSheet,
  View,
} from 'react-native';
import { AppText } from './AppText';
import { Button } from './Button';
import { GLOW_BOTTOM, GLOW_TOP } from './Dialog';
import { RadialFill } from './Gradient';
import { Spinner } from './Spinner';
import { colors, fonts } from './tokens';
import { u } from './scale';
import { t } from './i18n';

const rays = require('../assets/claim-rays.webp');

export type ClaimState = 'default' | 'loading' | 'claimed';

/** Claimed: the rays spin (one 51.4° step every 2 s, the glyph's own symmetry, so the loop is seamless)
 * and pop in; the reward drops in. */
function useClaimedMotion(claimed: boolean) {
  const spin = useRef(new Animated.Value(0)).current;
  const enter = useRef(new Animated.Value(claimed ? 1 : 0)).current;
  useEffect(() => {
    if (!claimed) {
      enter.setValue(0);
      return;
    }
    Animated.spring(enter, {
      toValue: 1,
      friction: 5,
      tension: 60,
      useNativeDriver: true,
    }).start();
    const loop = Animated.loop(
      Animated.timing(spin, {
        toValue: 1,
        duration: 2000,
        easing: Easing.linear,
        useNativeDriver: true,
      }),
    );
    loop.start();
    return () => loop.stop();
  }, [claimed, enter, spin]);
  return {
    rays: {
      opacity: enter.interpolate({
        inputRange: [0, 0.3, 1],
        outputRange: [0, 1, 1],
      }),
      transform: [
        {
          rotate: spin.interpolate({
            inputRange: [0, 1],
            outputRange: ['0deg', '51.43deg'],
          }),
        },
        { scale: enter },
      ],
    },
    reward: {
      transform: [
        {
          translateY: enter.interpolate({
            inputRange: [0, 1],
            outputRange: [-u(36), 0],
          }),
        },
      ],
    },
  };
}

/** Figma `Modal/Claim` 871:701: the reward claim modal of wearable campaigns over `Modal/Scrim`, on
 * the `Modal-Background` surface (`surface/modal` + the Dialog glows scaled to its 312×340 card).
 * `reward` is the campaign art. `loading` shows the spinner in its place and dims the button;
 * `claimed` adds the rays, disables the button and closes on a tap anywhere. */
export function ClaimDialog({
  visible,
  state,
  title,
  body,
  reward,
  buttonLabel,
  continueLabel,
  onClaim,
  onRequestClose,
}: {
  visible: boolean;
  state: ClaimState;
  title: string;
  body: string;
  reward: ReactNode;
  buttonLabel: string;
  /** The "Tap to continue" line under the card while `claimed`. */
  continueLabel: string;
  onClaim: () => void;
  onRequestClose: () => void;
}) {
  const claimed = state === 'claimed';
  const motion = useClaimedMotion(claimed);
  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      statusBarTranslucent
      onRequestClose={onRequestClose}
    >
      <View style={styles.scrim}>
        <Pressable
          accessibilityLabel={t('APP_DISMISS_DIALOG')}
          style={StyleSheet.absoluteFill}
          onPress={onRequestClose}
        />
        <View style={styles.column} pointerEvents="box-none">
          <View style={styles.card} accessibilityViewIsModal>
            <View style={styles.clip}>
              <RadialFill
                stops={GLOW_TOP}
                cx={1}
                cy={0}
                rxDp={u(126)}
                ryDp={u(125.68)}
              />
              <RadialFill
                stops={GLOW_BOTTOM}
                cx={0.5}
                cy={1.039}
                rxDp={u(205.92)}
                ryDp={u(92.6)}
              />
            </View>
            <View style={styles.text}>
              <AppText style={styles.title}>{title}</AppText>
              <AppText style={styles.body}>{body}</AppText>
            </View>
            <View style={styles.reward}>
              {claimed && (
                <Animated.Image
                  source={rays}
                  style={[styles.rays, motion.rays]}
                  resizeMode="contain"
                />
              )}
              {state === 'loading' ? (
                <View style={styles.spinner}>
                  <Spinner />
                </View>
              ) : (
                <Animated.View style={claimed && motion.reward}>
                  {reward}
                </Animated.View>
              )}
            </View>
            <Button
              tone="claim"
              fullWidth
              label={buttonLabel}
              loading={state === 'loading'}
              disabled={claimed}
              onPress={onClaim}
            />
            {claimed && (
              <Pressable
                accessibilityLabel={continueLabel}
                style={StyleSheet.absoluteFill}
                onPress={onRequestClose}
              />
            )}
          </View>
          {claimed && (
            <AppText style={styles.continue}>{continueLabel}</AppText>
          )}
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  scrim: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.black60,
  },
  column: { alignItems: 'center', gap: u(6) },
  card: {
    width: u(312),
    height: u(340),
    paddingTop: u(32),
    paddingBottom: u(30),
    paddingHorizontal: u(28),
    gap: u(12),
    alignItems: 'center',
    borderRadius: u(8),
    backgroundColor: colors.modal,
  },
  clip: {
    ...StyleSheet.absoluteFill,
    borderRadius: u(8),
    overflow: 'hidden',
  },
  text: { alignSelf: 'stretch', gap: u(8) },
  title: {
    fontFamily: fonts.bold,
    fontSize: u(22),
    color: colors.snow,
    textAlign: 'center',
  },
  body: {
    fontSize: u(13),
    lineHeight: u(18),
    color: colors.snow,
    textAlign: 'center',
  },
  reward: {
    flex: 1,
    alignSelf: 'stretch',
    alignItems: 'center',
    justifyContent: 'center',
  },
  /** The `Rays` node is 280×277 dp; the export keeps its blur margin (6% each side). */
  rays: { position: 'absolute', width: u(313.66), height: u(310.6) },
  /** The slot pads the spinner 35 top / 45 bottom: it sits 5 dp above centre. */
  spinner: { marginBottom: u(10) },
  continue: {
    fontFamily: fonts.medium,
    fontSize: u(15),
    color: colors.snow,
    textShadowColor: colors.black50,
    textShadowOffset: { width: 0, height: 0 },
    textShadowRadius: u(4.5),
  },
});
