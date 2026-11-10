import { useEffect, useState } from 'react';
import {
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
  Sheet,
  Spinner,
  u,
} from '@dcl/ui-native';
import { core } from '../core';
import { CameraFocus } from '../core/native';
import { useAvatar } from '../backpack/useAvatar';
import {
  applyPresetAvatar,
  fetchPresetAvatars,
  PresetAvatar,
} from './presetAvatars';
import { t } from '../i18n';

/** The window opens anyway after this long: a failed preset list must not leave the stage empty. */
const HOLD_TIMEOUT_MS = 5000;
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
  hold = false,
}: {
  inset?: boolean;
  stage?: 'full' | 'create' | 'naming';
  /** Keeps the avatar window closed: what the engine holds is not what the screen will show yet. */
  hold?: boolean;
}) {
  const avatar = useAvatar();
  return (
    <View style={styles.area}>
      <StageImage variant={stage === 'create' ? 'create' : 'full'} />
      {!avatar && (
        <View style={styles.loading}>
          <Spinner />
        </View>
      )}
      {!hold && (
        <AvatarHole
          style={
            stage === 'create'
              ? styles.createHole
              : stage === 'naming'
              ? styles.namingHole
              : inset
              ? styles.hole
              : styles.fill
          }
          focus={CameraFocus.Overall}
        />
      )}
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
        <Spinner size={30} />
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
  const insets = useSafeAreaInsets();
  const avatar = useAvatar();
  const [presets, setPresets] = useState<PresetAvatar[]>([]);
  // The window stays closed until the preset is applied, so the avatar the profile holds is never
  // drawn. The engine reports its avatar after preloading it: the preset needs that to apply.
  const [released, setReleased] = useState(false);
  useEffect(() => {
    let cancelled = false;
    // The edits resolve against the core's catalog: it has to be in before a preset is applied.
    Promise.all([fetchPresetAvatars(), core().loadAvatarCatalog()]).then(
      ([found]) => {
        if (!cancelled) {
          setPresets(found);
          setReleased(current => current || found.length === 0);
        }
      },
      () => !cancelled && setReleased(true),
    );
    const giveUp = setTimeout(() => setReleased(true), HOLD_TIMEOUT_MS);
    return () => {
      cancelled = true;
      clearTimeout(giveUp);
    };
  }, []);
  // An edit before the engine reports the avatar is dropped: apply again once it is there.
  const hasAvatar = avatar !== null;
  useEffect(() => {
    const preset = presets[selected];
    if (preset && hasAvatar) {
      applyPresetAvatar(preset);
      setReleased(true);
    }
  }, [presets, selected, hasAvatar]);

  return (
    <View style={styles.fill}>
      <PreviewArea inset={false} stage="create" hold={!released} />
      <Image source={createShadow} style={styles.avatarShadow} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('APP_BACK')}
        onPress={onBack}
        hitSlop={12}
        style={[styles.avatarBack, { top: Math.max(insets.top + u(4), u(43)) }]}
      >
        <Icon name="back" size={30} />
      </Pressable>
      <View style={styles.avatarSheet}>
        <Sheet
          footer={
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
          }
        >
          <View style={styles.avatarContent}>
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
        </Sheet>
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
  /** Figma AVATAR_CREATE 1450:7175: the avatar stands in a 672px tall box from y 165, feet on the shadow. */
  createHole: {
    position: 'absolute',
    left: 0,
    right: 0,
    top: u(82.5),
    height: u(336),
  },
  /** Figma AVATAR_NAMING 1897:13029: the avatar stands from y 435 to 1317, under the name field. */
  namingHole: {
    position: 'absolute',
    left: 0,
    right: 0,
    top: u(217.5),
    height: u(441),
  },
  loading: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
  },
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
  /** Same place as the backpack's arrow (`PortraitLayout`), insets included, so it does not move. */
  avatarBack: { position: 'absolute', left: u(24) },
  /** Figma's blurred ellipse under the feet (240×34 plus its blur), centred at (343, 820). */
  avatarShadow: {
    position: 'absolute',
    left: u(97.5),
    top: u(387.5),
    width: u(148),
    height: u(45),
  },
  /** Figma AVATAR_CREATE `Sheet`: the DS sheet with its handle, the title, 20 dp, the cards. */
  avatarSheet: { position: 'absolute', bottom: 0, left: 0, right: 0 },
  avatarContent: { gap: u(20) },
  avatarChooseTitle: {
    fontSize: u(18),
    lineHeight: u(17),
    textAlign: 'center',
  },
  /** Edge to edge of the sheet's 24 dp margin, so a dragged card leaves through the screen edge. */
  presetScroll: {
    alignSelf: 'stretch',
    marginHorizontal: -u(24),
    marginBottom: -u(3),
  },
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
  avatarFooterButtons: { flexDirection: 'row', gap: u(9) },
  avatarEditButton: { width: u(103) },
  avatarNextButton: { flex: 1 },
});
