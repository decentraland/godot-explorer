import { useState } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AvatarHole } from '../backpack/AvatarStage';
import { AppText, colors, fonts, Icon, LinearFill, RadialFill } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { AboutFields } from './AboutFields';
import { EditButton } from './EditButton';
import { EquippedRow } from './EquippedRow';
import { landscapeAbout } from './logic';
import { ProfileHeader } from './ProfileHeader';
import { ProfileSkeleton } from './ProfileSkeleton';
import type { EquippedItem, Profile } from './types';

const CARD_STOPS = [
  { offset: 0, color: '#431478' },
  { offset: 0.6, color: '#2E0A50' },
  { offset: 1, color: '#7A2BB0' },
];

export type ProfileLandscapeProps = {
  /** `null` draws the loading skeleton. */
  profile: Profile | null;
  /** Full-screen page (back chevron) instead of the floating card (X). */
  fullscreen?: boolean;
  initialExpanded?: boolean;
  onClose: () => void;
  onEdit: () => void;
  onCopyAddress: () => void;
  onEquippedPress?: (item: EquippedItem) => void;
};

/** Landscape profile: avatar hole on the left, header, About me (see more / less) and Equipped on the right. */
export function ProfileLandscape({
  profile,
  fullscreen = false,
  initialExpanded = false,
  onClose,
  onEdit,
  onCopyAddress,
  onEquippedPress,
}: ProfileLandscapeProps) {
  const [expanded, setExpanded] = useState(initialExpanded);
  const about = profile ? landscapeAbout(profile, expanded) : null;
  const toggleLabel = t(
    about?.toggle === 'less' ? 'PROFILE_SEE_LESS' : 'PROFILE_SEE_MORE',
  );
  const body = (
    <View style={styles.row}>
      <AvatarHole style={fullscreen ? styles.holeFull : styles.hole} />
      <ScrollView
        style={styles.flex}
        contentContainerStyle={[
          styles.content,
          fullscreen ? styles.contentFull : styles.contentCard,
        ]}
        showsVerticalScrollIndicator={false}
      >
        {profile && about ? (
          <>
            <View style={styles.header}>
              <ProfileHeader
                profile={profile}
                size="landscape"
                onCopyName={() => {}}
                onCopyAddress={onCopyAddress}
              />
              <View style={styles.buttons}>
                <EditButton compact onPress={onEdit} />
                {!fullscreen && (
                  <Pressable
                    accessibilityRole="button"
                    accessibilityLabel={t('PROFILE_CLOSE')}
                    onPress={onClose}
                    style={styles.close}
                  >
                    <Icon name="close" size={14} strokeWidth={2.4} />
                  </Pressable>
                )}
              </View>
            </View>
            {about.mode !== 'none' && (
              <View style={styles.about}>
                <AppText variant="label" style={styles.title}>
                  {t('PROFILE_ABOUT_ME')}
                </AppText>
                {about.showDescription && (
                  <AppText
                    variant="body"
                    numberOfLines={expanded ? undefined : fullscreen ? 3 : 2}
                    style={styles.description}
                  >
                    {profile.description}
                  </AppText>
                )}
                {about.keys.length > 0 && (
                  <View style={styles.fields}>
                    <AboutFields
                      profile={profile}
                      keys={about.keys}
                      columns={3}
                    />
                  </View>
                )}
                {about.toggle && (
                  <Pressable
                    accessibilityRole="button"
                    accessibilityLabel={toggleLabel}
                    onPress={() => setExpanded(!expanded)}
                    style={styles.toggle}
                  >
                    <AppText variant="label" style={styles.toggleText}>
                      {toggleLabel}
                    </AppText>
                  </Pressable>
                )}
              </View>
            )}
            {profile.equipped.length > 0 && (
              <View style={styles.equipped}>
                <AppText variant="label" style={styles.title}>
                  {t('PROFILE_EQUIPPED')}
                </AppText>
                <EquippedRow
                  items={profile.equipped}
                  onPress={onEquippedPress}
                />
              </View>
            )}
          </>
        ) : (
          <ProfileSkeleton />
        )}
      </ScrollView>
    </View>
  );

  if (fullscreen) {
    return (
      <View style={styles.page}>
        <LinearFill
          stops={[
            { offset: 0, color: '#6A1CB0' },
            { offset: 1, color: '#3A0A63' },
          ]}
          angle={120}
        />
        <RadialFill
          cx={0.28}
          cy={0.65}
          rx={0.22}
          ry={0.55}
          stops={[
            { offset: 0, color: '#C13DFF', opacity: 0.55 },
            { offset: 1, color: '#C13DFF', opacity: 0 },
          ]}
        />
        {body}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('PROFILE_BACK')}
          onPress={onClose}
          hitSlop={12}
          style={styles.back}
        >
          <Icon name="chevronLeft" size={24} />
        </Pressable>
      </View>
    );
  }
  return (
    <View style={styles.page}>
      <View style={styles.card}>
        <LinearFill stops={CARD_STOPS} angle={145} />
        {body}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  flex: { flex: 1 },
  page: { ...StyleSheet.absoluteFill, backgroundColor: 'rgba(20,4,34,0.7)' },
  card: {
    position: 'absolute',
    top: u(25),
    bottom: u(25),
    left: u(52),
    right: u(80),
    borderRadius: u(16),
    overflow: 'hidden',
  },
  row: { flex: 1, flexDirection: 'row' },
  hole: { width: u(168) },
  holeFull: { width: u(150), marginLeft: u(95) },
  content: { paddingBottom: u(16) },
  contentCard: { paddingTop: u(14), paddingRight: u(16) },
  contentFull: { paddingTop: u(28), paddingLeft: u(75), paddingRight: u(54) },
  header: { flexDirection: 'row', justifyContent: 'space-between' },
  buttons: { flexDirection: 'row', gap: u(10), alignItems: 'flex-start' },
  close: {
    width: u(29),
    height: u(29),
    borderRadius: u(8),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.black40,
  },
  about: { marginTop: u(18) },
  title: {
    fontSize: u(11),
    fontFamily: fonts.bold,
    color: colors.lavender,
    marginBottom: u(6),
  },
  description: { fontSize: u(12), lineHeight: u(16), color: colors.snow },
  fields: { marginTop: u(8) },
  toggle: { alignSelf: 'flex-end', marginTop: u(6) },
  toggleText: {
    fontFamily: fonts.bold,
    fontSize: u(10),
    color: colors.periwinkle,
    textDecorationLine: 'underline',
  },
  equipped: { marginTop: u(8) },
  back: { position: 'absolute', left: u(56), top: u(18) },
});
