import { CameraFocus } from '../core/native';
import { ReactNode, useState } from 'react';
import {
  Pressable,
  ScrollView,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { AvatarHole } from '../backpack/AvatarStage';
import {
  AppText,
  BottomSheet,
  colors,
  Icon,
  radii,
  Snap,
  snapHeights,
  u,
} from '@dcl/ui-native';
import { t } from '../i18n';
import { AboutFields } from './AboutFields';
import { EditButton } from './EditButton';
import { EquippedRow } from './EquippedRow';
import { LinkChips } from './LinkChips';
import { aboutMode, filledAboutKeys } from './logic';
import { Backdrop, ProfileBackdrop } from './ProfileBackdrop';
import { ProfileHeader } from './ProfileHeader';
import { ProfileSkeleton } from './ProfileSkeleton';
import type { EquippedItem, Profile } from './types';

const PEEK_DP = 124;
// Half is 584px from the top of the 1600px frame with About me, 718px without it.
const HALF_WITH_ABOUT = 0.635;
const HALF_NO_ABOUT = 0.551;

export type ProfileSheetProps = {
  /** `null` draws the loading skeleton. */
  profile: Profile | null;
  snap: Snap;
  onSnapChange: (snap: Snap) => void;
  footer?: ReactNode;
  /** False while another screen covers the tab: the Godot avatar draws above RN and must not show through. */
  avatarVisible?: boolean;
  backdrop?: Backdrop;
  onEdit: () => void;
  onCopyName: () => void;
  onCopyAddress: () => void;
  onEquippedPress?: (item: EquippedItem) => void;
};

/** Own profile over the avatar: bottom sheet that rests at peek, half or full. */
export function ProfileSheet({
  profile,
  snap,
  onSnapChange,
  footer,
  avatarVisible = true,
  backdrop,
  onEdit,
  onCopyName,
  onCopyAddress,
  onEquippedPress,
}: ProfileSheetProps) {
  const { height } = useWindowDimensions();
  const [area, setArea] = useState(height);
  const hasAbout = profile ? aboutMode(profile) !== 'none' : true;
  const screen = snapHeights(height, {
    peek: u(PEEK_DP),
    half: hasAbout ? HALF_WITH_ABOUT : HALF_NO_ABOUT,
    full: 1,
  });
  // The Figma snaps are measured on the whole frame; the sheet lives above the nav bar, so each
  // keeps its on-screen top by giving up the part the bar covers.
  const covered = height - area;
  const config = {
    peek: Math.max(screen.peek - covered, 0),
    half: (screen.half - covered) / area,
    full: 1,
  };
  const holeHeight = height - screen[snap];
  return (
    <View style={styles.root}>
      <ProfileBackdrop variant={backdrop} />
      <View
        style={styles.area}
        onLayout={event => setArea(event.nativeEvent.layout.height)}
      >
        {snap !== 'full' && avatarVisible && (
          <AvatarHole
            focus={CameraFocus.Head}
            style={[styles.hole, { height: holeHeight }]}
          />
        )}
        <BottomSheet
          snap={snap}
          onSnapChange={onSnapChange}
          config={config}
          containerHeight={area}
        >
          <ScrollView
            contentContainerStyle={styles.content}
            showsVerticalScrollIndicator={false}
          >
            {snap === 'full' && (
              <View style={styles.close}>
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={t('PROFILE_CLOSE')}
                  onPress={() => onSnapChange('half')}
                  hitSlop={8}
                  style={({ pressed }) => [
                    styles.closeButton,
                    pressed && styles.closeButtonPressed,
                  ]}
                >
                  <Icon name="close" size={20} />
                </Pressable>
              </View>
            )}
            {profile ? (
              <Content
                profile={profile}
                onEdit={onEdit}
                onCopyName={onCopyName}
                onCopyAddress={onCopyAddress}
                onEquippedPress={onEquippedPress}
              />
            ) : (
              <ProfileSkeleton />
            )}
          </ScrollView>
        </BottomSheet>
      </View>
      {footer}
    </View>
  );
}

function Content({
  profile,
  onEdit,
  onCopyName,
  onCopyAddress,
  onEquippedPress,
}: Pick<
  ProfileSheetProps,
  'onEdit' | 'onCopyName' | 'onCopyAddress' | 'onEquippedPress'
> & { profile: Profile }) {
  const keys = filledAboutKeys(profile);
  const hasDescription = profile.description.trim().length > 0;
  return (
    <>
      <ProfileHeader
        profile={profile}
        size="portrait"
        onCopyName={onCopyName}
        onCopyAddress={onCopyAddress}
      />
      <View style={styles.edit}>
        <EditButton onPress={onEdit} />
      </View>
      {(hasDescription || keys.length > 0) && (
        <Section title={t('PROFILE_ABOUT_ME')}>
          {hasDescription && (
            <AppText variant="body" style={styles.description}>
              {profile.description}
            </AppText>
          )}
          <View style={styles.fields}>
            <AboutFields profile={profile} keys={keys} columns={2} />
          </View>
        </Section>
      )}
      {profile.links.length > 0 && (
        <Section title={t('PROFILE_LINKS_CAPS')}>
          <LinkChips links={profile.links} />
        </Section>
      )}
      {profile.equipped.length > 0 && (
        <Section title={t('PROFILE_EQUIPPED')}>
          <EquippedRow items={profile.equipped} onPress={onEquippedPress} />
        </Section>
      )}
    </>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <View style={styles.section}>
      <AppText variant="label" style={styles.sectionTitle}>
        {title}
      </AppText>
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
  hole: { position: 'absolute', top: 0, left: 0, right: 0 },
  /** Clipped so the sheet stops at the nav bar instead of running under it. */
  area: { flex: 1, overflow: 'hidden' },
  content: { paddingHorizontal: u(24), paddingBottom: u(24) },
  close: { marginBottom: u(12), alignSelf: 'flex-start' },
  closeButton: {
    width: u(36),
    height: u(36),
    borderRadius: radii.button,
    backgroundColor: colors.purple550,
    alignItems: 'center',
    justifyContent: 'center',
  },
  closeButtonPressed: { opacity: 0.8 },
  edit: { marginTop: u(26) },
  section: { marginTop: u(30) },
  sectionTitle: {
    fontSize: u(12),
    fontFamily: 'Inter-Bold',
    color: colors.lavender,
    marginBottom: u(8),
  },
  description: { fontSize: u(13), lineHeight: u(19), color: colors.snow },
  fields: { marginTop: u(18) },
});
