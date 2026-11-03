import { useEffect, useState } from 'react';
import {
  ActivityIndicator,
  Image,
  ImageBackground,
  Pressable,
  ScrollView,
  StyleSheet,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AvatarHole } from '../backpack/AvatarStage';
import { AppText, colors, Icon, LinearFill, Button, u } from '@dcl/ui-native';
import { useAvatar } from '../backpack/useAvatar';
import {
  applyPresetAvatar,
  fetchPresetAvatars,
  PresetAvatar,
} from './presetAvatars';
import { t } from '../i18n';

const stageBackground = require('../assets/avatar-stage-background.png');

/** Figma `Background/Avatar` over the brand purple, as the HIFI avatar screens compose it.
 * Stretched, not covered: the glow and floor keep the frame's vertical proportions on every
 * phone, and a few percent of anisotropy is invisible on a blurred gradient. */
export function StageImage() {
  return (
    <ImageBackground
      source={stageBackground}
      style={StyleSheet.absoluteFill}
      resizeMode="stretch"
    />
  );
}

/**
 * Stage + the single Godot avatar window, with a spinner behind it until the avatar arrives. `inset`
 * leaves room for a full-screen screen's title and buttons; a screen that sizes the area itself
 * turns it off.
 */
export function PreviewArea({ inset = true }: { inset?: boolean }) {
  const avatar = useAvatar();
  return (
    <View style={styles.area}>
      <StageImage />
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

function PresetCard({
  preset,
  selected,
  onPress,
}: {
  preset: PresetAvatar | null;
  selected: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      disabled={!preset}
      onPress={onPress}
      style={[styles.presetCard, selected && styles.presetCardSelected]}
    >
      {preset ? (
        <Image
          source={{ uri: preset.thumbnail }}
          style={styles.presetThumbnail}
          resizeMode="cover"
        />
      ) : (
        <ActivityIndicator color={colors.snow} />
      )}
    </Pressable>
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
  const [presets, setPresets] = useState<PresetAvatar[]>([]);
  const [selected, setSelected] = useState(0);
  useEffect(() => {
    let cancelled = false;
    fetchPresetAvatars().then(found => {
      if (!cancelled) {
        setPresets(found);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);
  useEffect(() => {
    const preset = presets[selected];
    if (preset) {
      applyPresetAvatar(preset, avatar);
    }
    // Applying only when the selection (or the fetched list) changes, not on every avatar event.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [presets, selected]);

  return (
    <View style={styles.fill}>
      <PreviewArea inset={false} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('APP_BACK')}
        onPress={onBack}
        hitSlop={12}
        style={[styles.avatarBack, { top: insets.top + u(20) }]}
      >
        <Icon name="back" size={30} />
      </Pressable>
      <View style={styles.avatarSheet}>
        <View style={styles.avatarDetail}>
          <LinearFill
            stops={[
              { offset: 0, color: colors.purple450 },
              { offset: 1, color: colors.purple500 },
            ]}
          />
          <AppText variant="heading" style={styles.avatarChooseTitle}>
            {t('FLOW_AVATAR_CHOOSE_LOOK')}
          </AppText>
          <ScrollView
            horizontal
            showsHorizontalScrollIndicator={false}
            contentContainerStyle={styles.presetRow}
          >
            {(presets.length ? presets : [null, null, null, null]).map(
              (preset, i) => (
                <PresetCard
                  key={preset?.id ?? i}
                  preset={preset}
                  selected={i === selected}
                  onPress={() => setSelected(i)}
                />
              ),
            )}
          </ScrollView>
        </View>
        <View
          style={[
            styles.avatarFooter,
            { paddingBottom: insets.bottom + u(20) },
          ]}
        >
          <View style={styles.avatarFooterButtons}>
            <View style={styles.avatarEditButton}>
              <Button
                big
                tone="dark"
                label={t('FLOW_AVATAR_EDIT')}
                disabled={!avatar}
                onPress={onCustomize}
              />
            </View>
            <View style={styles.avatarNextButton}>
              <Button
                big
                label={t('FLOW_AVATAR_NEXT')}
                disabled={!avatar}
                onPress={onContinue}
              />
            </View>
          </View>
        </View>
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
        <Button
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
  centered: { textAlign: 'center' },
  avatarBack: { position: 'absolute', left: u(24) },
  avatarSheet: { position: 'absolute', bottom: 0, left: 0, right: 0 },
  avatarDetail: {
    alignItems: 'center',
    gap: u(22),
    paddingTop: u(20),
    paddingBottom: u(15),
    paddingHorizontal: u(24),
    borderTopLeftRadius: u(12),
    borderTopRightRadius: u(12),
    overflow: 'hidden',
  },
  avatarChooseTitle: {
    fontSize: u(18),
    lineHeight: u(18),
    textAlign: 'center',
  },
  presetRow: { flexDirection: 'row', gap: u(12) },
  presetCard: {
    width: u(80),
    height: u(180),
    borderRadius: u(8),
    backgroundColor: colors.black40,
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
  },
  presetCardSelected: {
    backgroundColor: 'rgba(0,0,0,0.7)',
    borderWidth: u(2),
    borderColor: '#FFBC5B',
  },
  presetThumbnail: { width: '100%', height: '100%' },
  avatarFooter: {
    backgroundColor: colors.purple500,
    paddingTop: u(9),
    paddingHorizontal: u(24),
  },
  avatarFooterButtons: { flexDirection: 'row', gap: u(9) },
  avatarEditButton: { flex: 1 },
  avatarNextButton: { flex: 2 },
});
