import { ActivityIndicator, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AvatarHole, StageBackground } from '../backpack/AvatarStage';
import { AppText, PrimaryButton } from '../ds';
import { core } from '../core';
import { Control } from '../core/native';
import { useAvatar } from '../backpack/useAvatar';
import { t } from '../i18n';
import { u } from '../ds/scale';

/**
 * Stage + the single Godot avatar window, with a spinner behind it until the avatar arrives. `inset`
 * leaves room for a full-screen screen's title and buttons; a screen that sizes the area itself
 * turns it off.
 */
export function PreviewArea({ inset = true }: { inset?: boolean }) {
  const avatar = useAvatar();
  return (
    <View style={styles.area}>
      <StageBackground />
      {!avatar && (
        <View style={styles.loading}>
          <ActivityIndicator color="#FCFCFC" />
          <AppText variant="caption" style={styles.loadingText}>
            {t('FLOW_AVATAR_LOADING')}
          </AppText>
        </View>
      )}
      <AvatarHole style={inset ? styles.hole : styles.fill} />
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
  const avatar = useAvatar();
  return (
    <View style={styles.fill}>
      <PreviewArea />
      <View style={[styles.top, { paddingTop: insets.top + u(16) }]}>
        <AppText variant="heading" style={styles.centered}>
          {t('FLOW_AVATAR_TITLE')}
        </AppText>
        <AppText variant="body" style={styles.centered}>
          {t('FLOW_AVATAR_BODY')}
        </AppText>
      </View>
      <View style={[styles.bottom, { paddingBottom: insets.bottom + 24 }]}>
        <View style={styles.row}>
          <View style={styles.flex}>
            <PrimaryButton
              label={t('FLOW_AVATAR_RANDOMIZE')}
              tone="secondary"
              disabled={!avatar}
              onPress={() => {
                // The core draws from the wearable catalog, so it has to be there first.
                core()
                  .loadAvatarCatalog()
                  .then(() => core().control(new Control.RandomizeAvatar()))
                  .catch(() => {});
              }}
            />
          </View>
          <View style={styles.flex}>
            <PrimaryButton
              label={t('FLOW_AVATAR_CUSTOMIZE')}
              tone="secondary"
              disabled={!avatar}
              onPress={onCustomize}
            />
          </View>
        </View>
        <PrimaryButton
          label={t('FLOW_CONTINUE')}
          disabled={!avatar}
          onPress={onContinue}
        />
        <PrimaryButton label={t('APP_BACK')} tone="ghost" onPress={onBack} />
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
  const avatar = useAvatar();
  return (
    <View style={styles.fill}>
      <PreviewArea />
      <View style={[styles.top, { paddingTop: insets.top + u(16) }]}>
        <AppText variant="heading" style={styles.centered}>
          {t('FLOW_WELCOME_BACK')}
        </AppText>
        <AppText variant="heading" style={styles.centered}>
          {name}
        </AppText>
      </View>
      <View style={[styles.bottom, { paddingBottom: insets.bottom + 24 }]}>
        <PrimaryButton
          label={t('FLOW_LETS_GO')}
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
