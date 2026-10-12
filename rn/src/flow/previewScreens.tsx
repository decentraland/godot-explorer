import { useEffect } from 'react';
import { ActivityIndicator, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AvatarHole, StageBackground } from '../backpack/AvatarStage';
import { AppText, PrimaryButton } from '../ds';
import { ensureEngine, send, useEngine } from '../engine';
import { u } from '../ds/scale';

/** Stage + the single Godot avatar window, with a spinner behind it until the avatar arrives. */
export function PreviewArea() {
  const { avatar } = useEngine();
  useEffect(() => {
    ensureEngine();
  }, []);
  return (
    <View style={styles.area}>
      <StageBackground />
      {!avatar && (
        <View style={styles.loading}>
          <ActivityIndicator color="#FCFCFC" />
          <AppText variant="caption" style={styles.loadingText}>
            Getting your avatar ready…
          </AppText>
        </View>
      )}
      <AvatarHole style={styles.hole} />
    </View>
  );
}

export function AvatarCreateScreen({
  onCustomize,
  onContinue,
  onBack,
}: {
  onCustomize: () => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const insets = useSafeAreaInsets();
  const { avatar } = useEngine();
  return (
    <View style={styles.fill}>
      <PreviewArea />
      <View style={[styles.top, { paddingTop: insets.top + u(16) }]}>
        <AppText variant="heading" style={styles.centered}>
          Create your avatar
        </AppText>
        <AppText variant="body" style={styles.centered}>
          Shuffle until you like it, then make it yours.
        </AppText>
      </View>
      <View style={[styles.bottom, { paddingBottom: insets.bottom + 24 }]}>
        <View style={styles.row}>
          <View style={styles.flex}>
            <PrimaryButton
              label="Randomize"
              tone="secondary"
              disabled={!avatar}
              onPress={() => send({ type: 'randomize_avatar' })}
            />
          </View>
          <View style={styles.flex}>
            <PrimaryButton
              label="Customize"
              tone="secondary"
              disabled={!avatar}
              onPress={onCustomize}
            />
          </View>
        </View>
        <PrimaryButton
          label="Continue"
          disabled={!avatar}
          onPress={onContinue}
        />
        <PrimaryButton label="Back" tone="ghost" onPress={onBack} />
      </View>
    </View>
  );
}

export function WelcomeBackScreen({
  name,
  onContinue,
}: {
  name: string;
  onContinue: () => void;
}) {
  const insets = useSafeAreaInsets();
  const { avatar } = useEngine();
  return (
    <View style={styles.fill}>
      <PreviewArea />
      <View style={[styles.top, { paddingTop: insets.top + u(16) }]}>
        <AppText variant="heading" style={styles.centered}>
          Welcome back,
        </AppText>
        <AppText variant="heading" style={styles.centered}>
          {name}
        </AppText>
      </View>
      <View style={[styles.bottom, { paddingBottom: insets.bottom + 24 }]}>
        <PrimaryButton
          label="Let's go"
          disabled={!avatar}
          onPress={onContinue}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  flex: { flex: 1 },
  area: { ...StyleSheet.absoluteFill },
  hole: { flex: 1, marginTop: u(130), marginBottom: u(250) },
  loading: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
  },
  loadingText: { marginTop: u(8) },
  top: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    paddingHorizontal: u(24),
  },
  bottom: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    paddingHorizontal: u(24),
    gap: u(12),
  },
  row: { flexDirection: 'row', gap: u(12) },
  centered: { textAlign: 'center' },
});
