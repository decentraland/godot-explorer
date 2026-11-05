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
import {
  AppText,
  colors,
  Icon,
  lightRainbow,
  LinearFill,
  Button,
  u,
} from '@dcl/ui-native';
import { useAvatar } from '../backpack/useAvatar';
import {
  applyPresetAvatar,
  fetchPresetAvatars,
  PresetAvatar,
} from './presetAvatars';
import { t } from '../i18n';

const stageBackground = require('../assets/avatar-stage-background.png');
const createBackground = require('../assets/avatar-create-background.png');
const createShadow = require('../assets/avatar-create-shadow.png');

/** Figma `Background/Avatar` over the brand background, as the HIFI avatar screens compose it:
 * `full` fills the screen (Naming, Welcome Back), `create` is the Glow background with the stage
 * scaled into the top half, above the Selection sheet. Stretched, not covered: the glow and floor
 * keep the frame's vertical proportions on every phone, and a few percent of anisotropy is
 * invisible on a blurred gradient. */
export function StageImage({
  variant = 'full',
}: {
  variant?: 'full' | 'create';
}) {
  return (
    <ImageBackground
      source={variant === 'create' ? createBackground : stageBackground}
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
export function PreviewArea({
  inset = true,
  stage = 'full',
}: {
  inset?: boolean;
  stage?: 'full' | 'create';
}) {
  const avatar = useAvatar();
  return (
    <View style={styles.area}>
      <StageImage variant={stage} />
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
      {selected && (
        <LinearFill
          stops={lightRainbow}
          start={{ x: -0.1, y: 0.2 }}
          end={{ x: 1.1, y: 0.8 }}
          stroke={u(2)}
          radius={u(8)}
        />
      )}
    </Pressable>
  );
}

export function AvatarCreateScreen({
  onCustomize,
  selected,
  onSelect,
  onContinue,
  onBack,
}: {
  onCustomize: () => void;
  /** Lives in the flow, so coming back from EDIT or the name returns to the same preset. */
  selected: number;
  onSelect: (index: number) => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const avatar = useAvatar();
  const [presets, setPresets] = useState<PresetAvatar[]>([]);
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
      <PreviewArea inset={false} stage="create" />
      <Image source={createShadow} style={styles.avatarShadow} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('APP_BACK')}
        onPress={onBack}
        hitSlop={12}
        style={styles.avatarBack}
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
            alwaysBounceHorizontal
            overScrollMode="always"
            style={styles.presetScroll}
            contentContainerStyle={styles.presetRow}
          >
            {(presets.length ? presets : [null, null, null, null]).map(
              (preset, i) => (
                <PresetCard
                  key={preset?.id ?? i}
                  preset={preset}
                  selected={i === selected}
                  onPress={() => onSelect(i)}
                />
              ),
            )}
          </ScrollView>
        </View>
        <View style={styles.avatarFooter}>
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
  avatarBack: { position: 'absolute', left: u(24), top: u(43) },
  /** Figma's blurred ellipse under the feet (240×34 plus its blur), centred at (343, 820). */
  avatarShadow: {
    position: 'absolute',
    left: u(97.5),
    top: u(387.5),
    width: u(148),
    height: u(45),
  },
  avatarSheet: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    borderTopLeftRadius: u(12),
    borderTopRightRadius: u(12),
    boxShadow: [
      { offsetX: 0, offsetY: -u(2), blurRadius: u(1), color: colors.black25 },
    ],
  },
  avatarDetail: {
    alignItems: 'center',
    gap: u(19.5),
    paddingTop: u(20),
    paddingBottom: u(14),
    borderTopLeftRadius: u(12),
    borderTopRightRadius: u(12),
    overflow: 'hidden',
  },
  avatarChooseTitle: {
    marginTop: u(2.5),
    fontSize: u(18),
    lineHeight: u(17),
    textAlign: 'center',
  },
  /** Edge to edge, so a dragged card leaves through the screen edge, not the margin. */
  presetScroll: { alignSelf: 'stretch' },
  presetRow: {
    flexDirection: 'row',
    gap: u(12),
    paddingHorizontal: u(24),
    paddingBottom: u(3),
  },
  presetCard: {
    width: u(80),
    height: u(180),
    borderRadius: u(8),
    backgroundColor: colors.black40,
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
    boxShadow: [
      { offsetX: 0, offsetY: u(2), blurRadius: u(1), color: colors.black25 },
    ],
  },
  presetCardSelected: { backgroundColor: colors.black70 },
  presetThumbnail: { width: '100%', height: '100%' },
  avatarFooter: {
    backgroundColor: colors.purple500,
    paddingTop: u(9),
    paddingHorizontal: u(24),
    paddingBottom: u(43),
  },
  avatarFooterButtons: { flexDirection: 'row', gap: u(9) },
  avatarEditButton: { width: u(103) },
  avatarNextButton: { flex: 1 },
});
