import { useEffect, useRef, useState } from 'react';
import { Animated, Image, Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, fonts, Dialog, Icon, LinearFill, u } from '../ds';
import type { LoadingProgress } from '../hud/types';
import { t } from '../i18n';
import { LoadingProgressRow } from './LoadingProgressRow';
import { LoadingPhase, Tip, TIP_ROTATE_MS, tipAt } from './loadingModel';
import { PlaceBackdrop } from './PlaceBackdrop';
import { TipCard } from './TipCard';

const logo = require('../assets/logo.png');

export type CoverPlace = { title: string; creator?: string; image?: string };

const SIDE_SCRIM = [
  { offset: 0, color: '#2a0d44', opacity: 1 },
  { offset: 0.2, color: '#2a0d44', opacity: 0.95 },
  { offset: 0.65, color: '#2a0d44', opacity: 0 },
];
const FADE_MS = 300;

/** Full-screen loading cover; `place` undefined means the place data has not arrived yet. */
export function LoadingCover({
  place,
  phase,
  progress,
  tip,
  startTip = 0,
  onClose,
  onReload,
  onStartAnyway,
  onGone,
}: {
  place?: CoverPlace;
  phase: LoadingPhase;
  progress: LoadingProgress;
  /** Pins one tip; otherwise tips rotate. */
  tip?: Tip;
  startTip?: number;
  onClose: () => void;
  onReload: () => void;
  onStartAnyway: () => void;
  onGone?: () => void;
}) {
  const insets = useSafeAreaInsets();
  const [tipIndex, setTipIndex] = useState(startTip);
  const opacity = useRef(new Animated.Value(1)).current;
  const [gone, setGone] = useState(false);

  useEffect(() => {
    if (tip) {
      return undefined;
    }
    const timer = setInterval(() => setTipIndex(i => i + 1), TIP_ROTATE_MS);
    return () => clearInterval(timer);
  }, [tip]);

  useEffect(() => {
    if (phase !== 'finished') {
      opacity.setValue(1);
      setGone(false);
      return;
    }
    Animated.timing(opacity, {
      toValue: 0,
      duration: FADE_MS,
      useNativeDriver: true,
    }).start(({ finished }) => {
      if (finished) {
        setGone(true);
        onGone?.();
      }
    });
  }, [phase, opacity, onGone]);

  if (gone) {
    return null;
  }
  const title = place?.title || t('LOADING_ENJOY_EXPERIENCE');
  const left = Math.max(insets.left, u(40));
  return (
    <Animated.View
      style={[styles.root, { opacity }]}
      pointerEvents={phase === 'finished' ? 'none' : 'auto'}
    >
      <PlaceBackdrop />
      {place?.image ? (
        <Image
          source={{ uri: place.image }}
          style={StyleSheet.absoluteFill}
          blurRadius={24}
        />
      ) : null}
      <View
        style={[StyleSheet.absoluteFill, styles.tint]}
        pointerEvents="none"
      />
      <LinearFill stops={SIDE_SCRIM} angle={90} />

      <View style={[styles.column, { top: Math.max(insets.top, u(16)) + u(32), left }]}>
        <Image source={logo} style={styles.logo} />
        <AppText variant="heading" numberOfLines={3} style={styles.title}>
          {title}
        </AppText>
        {place?.creator ? (
          <AppText variant="body" style={styles.creator}>
            {t('LOADING_BY')}{' '}
            <AppText
              variant="body"
              style={[styles.creator, styles.creatorName]}
            >
              {place.creator}
            </AppText>
          </AppText>
        ) : null}
        <View style={styles.tip}>
          <TipCard tip={tip ?? tipAt(tipIndex)} />
        </View>
      </View>

      <View
        style={[
          styles.bottom,
          { left, right: left, bottom: Math.max(insets.bottom, u(22.5)) },
        ]}
      >
        <LoadingProgressRow progress={progress} />
      </View>

      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('LOADING_CLOSE')}
        onPress={onClose}
        style={[
          styles.close,
          { top: Math.max(insets.top, u(16)), right: Math.max(insets.right, u(16)) },
        ]}
      >
        <Icon name="close" size={14} color={colors.snow} />
      </Pressable>

      <Dialog
        visible={phase === 'slow'}
        title={t('MODAL_SCENE_TIMEOUT_TITLE')}
        body={t('MODAL_SCENE_TIMEOUT_BODY')}
        onRequestClose={onStartAnyway}
        buttons={[
          {
            label: t('MODAL_SCENE_TIMEOUT_PRIMARY'),
            tone: 'secondary',
            onPress: onReload,
          },
          { label: t('MODAL_SCENE_TIMEOUT_SECONDARY'), onPress: onStartAnyway },
        ]}
      />
      <Dialog
        visible={phase === 'failed'}
        title={t('LOADING_FAILED_TITLE')}
        body={t('LOADING_FAILED_BODY')}
        icon="warning"
        onRequestClose={onClose}
        buttons={[
          {
            label: t('LOADING_FAILED_BACK'),
            tone: 'secondary',
            onPress: onClose,
          },
          { label: t('LOADING_FAILED_RETRY'), onPress: onReload },
        ]}
      />
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple550 },
  tint: { backgroundColor: 'rgba(70,15,111,0.25)' },
  column: { position: 'absolute' },
  logo: {
    width: u(29),
    height: u(29),
    borderRadius: u(14.5),
    marginBottom: u(13),
  },
  title: {
    maxWidth: u(180),
    fontSize: u(32),
    lineHeight: u(39),
    color: colors.snow,
  },
  creator: {
    marginTop: u(6),
    fontSize: u(13),
    lineHeight: u(16),
    color: colors.snow,
  },
  creatorName: { fontFamily: fonts.semibold },
  tip: { marginTop: u(30) },
  bottom: { position: 'absolute' },
  close: {
    position: 'absolute',
    width: u(30),
    height: u(30),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'rgba(0,0,0,0.5)',
  },
});
