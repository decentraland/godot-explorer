import { useEffect, useState } from 'react';
import { useWindowDimensions } from 'react-native';
import { useNav } from '../app/NavProvider';
import type { ScreenProps } from '../app/registry';
import { EmptyState, Snap, useToast } from '../ds';
import { send } from '../engine';
import { t } from '../i18n';
import { getProfileApi } from './api';
import { EditProfile } from './EditProfile';
import { ProfileLandscape } from './ProfileLandscape';
import { ProfileSheet } from './ProfileSheet';
import { loadProfile, saveProfile, useProfile } from './store';
import type { EquippedItem, ProfileDeployPayload } from './types';

const playEmote = (item: EquippedItem) => {
  if (item.isEmote) {
    send({ type: 'play_emote', urn: item.id });
  }
};

/** The Profile tab: bottom sheet over the avatar in portrait, the full-screen page in landscape. */
export function ProfileTab({ footer }: ScreenProps<'profile'>) {
  const state = useProfile();
  const nav = useNav();
  const toast = useToast();
  const { width, height } = useWindowDimensions();
  const [snap, setSnap] = useState<Snap>('half');

  useEffect(() => {
    loadProfile();
  }, []);

  if (state.status === 'error') {
    return (
      <EmptyState
        icon="wifiOff"
        title={t('PROFILE_LOAD_FAILED')}
        actionLabel={t('COMMON_TRY_AGAIN')}
        onAction={loadProfile}
      />
    );
  }
  const profile = state.profile;
  const copy = (text: string, message: string) => {
    getProfileApi().copyText(text);
    toast(message, { icon: 'check' });
  };
  const common = {
    profile,
    onEdit: () => nav.push({ name: 'profileEdit' }),
    onEquippedPress: playEmote,
  };
  const address = () => copy(profile?.address ?? '', t('TOAST_ADDRESS_COPIED'));
  if (width > height) {
    return (
      <ProfileLandscape
        {...common}
        fullscreen
        onClose={() => nav.setTab('explore')}
        onCopyAddress={address}
      />
    );
  }
  return (
    <ProfileSheet
      {...common}
      snap={snap}
      onSnapChange={setSnap}
      avatarVisible={nav.route.name === 'home'}
      footer={footer}
      onCopyName={() =>
        copy(
          profile ? `${profile.name}#${profile.tag}` : '',
          t('TOAST_NAME_COPIED'),
        )
      }
      onCopyAddress={address}
    />
  );
}

/** Edit Profile as a pushed route: SAVE closes at once and deploys in the background. */
export function ProfileEditRoute() {
  const state = useProfile();
  const nav = useNav();
  const toast = useToast();
  const [names, setNames] = useState<string[]>([]);

  useEffect(() => {
    let live = true;
    getProfileApi()
      .getNames()
      .then(found => live && setNames(found))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, []);

  if (!state.profile) {
    return null;
  }
  const save = (patch: ProfileDeployPayload) => {
    nav.pop();
    saveProfile(patch).then(ok =>
      toast(ok ? t('PROFILE_SAVED') : t('PROFILE_SAVE_FAILED'), {
        icon: ok ? 'check' : 'warning',
      }),
    );
  };
  return (
    <EditProfile
      profile={state.profile}
      names={names}
      onClose={nav.pop}
      onSave={save}
    />
  );
}
