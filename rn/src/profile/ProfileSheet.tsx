import { ReactNode } from 'react';
import {
  ScrollView,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AvatarHole } from '../backpack/AvatarStage';
import {
  AppText,
  BottomSheet,
  colors,
  IconButton,
  layout,
  Snap,
  snapHeights,
} from '../ds';
import { u } from '../ds/scale';
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
  const insets = useSafeAreaInsets();
  const hasAbout = profile ? aboutMode(profile) !== 'none' : true;
  const config = {
    peek: u(PEEK_DP),
    half: hasAbout ? HALF_WITH_ABOUT : HALF_NO_ABOUT,
    full: 1,
  };
  const holeHeight = height - snapHeights(height, config)[snap];
  return (
    <View style={styles.root}>
      <ProfileBackdrop variant={backdrop} />
      {snap !== 'full' && avatarVisible && (
        <AvatarHole
          focus="head"
          style={[styles.hole, { height: holeHeight }]}
        />
      )}
      <BottomSheet
        snap={snap}
        onSnapChange={onSnapChange}
        config={config}
        containerHeight={height}
      >
        <ScrollView
          contentContainerStyle={[
            styles.content,
            { paddingBottom: layout.navHeight + insets.bottom + u(24) },
          ]}
          showsVerticalScrollIndicator={false}
        >
          {snap === 'full' && (
            <View style={styles.close}>
              <IconButton
                icon="close"
                size={36}
                label={t('PROFILE_CLOSE')}
                onPress={() => onSnapChange('half')}
              />
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
      {footer ? <View style={styles.footer}>{footer}</View> : null}
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
  footer: { position: 'absolute', left: 0, right: 0, bottom: 0 },
  content: { paddingHorizontal: u(24), paddingTop: u(8) },
  close: { marginBottom: u(12), alignSelf: 'flex-start' },
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
