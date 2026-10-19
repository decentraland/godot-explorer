import { useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { BottomNav, colors, Dropdown, Snap } from '../ds';
import { u } from '../ds/scale';
import { registerPreview } from '../gallery/registry';
import { t } from '../i18n';
import { EditProfile, EditProfileProps } from './EditProfile';
import { MOCK_NAMES, MOCK_PROFILE } from './mockData';
import {
  GENDERS,
  ORIENTATIONS,
  OptionId,
  PRONOUNS,
  RELATIONSHIPS,
} from './options';
import { Backdrop } from './ProfileBackdrop';
import { ProfileLandscape } from './ProfileLandscape';
import { ProfileSheet } from './ProfileSheet';
import type { Profile } from './types';

const noop = () => {};
const nav = <BottomNav active="profile" bottomInset={0} onChange={noop} />;

const noAbout: Profile = {
  ...MOCK_PROFILE,
  description: '',
  country: '',
  language: '',
  pronouns: '',
  gender: '',
  sexualOrientation: '',
  relationshipStatus: '',
  employmentStatus: '',
  profession: '',
  realName: '',
  hobbies: '',
  links: [],
};
const dataOnly = (count: number): Profile => ({
  ...noAbout,
  country: 'Argentina',
  language: 'English',
  pronouns: 'He / Him',
  ...(count > 3
    ? {
        gender: 'Male',
        relationshipStatus: 'Single',
        sexualOrientation: 'Bisexual',
      }
    : {}),
});

function SheetPreview({
  profile = MOCK_PROFILE,
  snap: initial,
  backdrop,
}: {
  profile?: Profile;
  snap: Snap;
  backdrop?: Backdrop;
}) {
  const [snap, setSnap] = useState(initial);
  return (
    <ProfileSheet
      profile={profile}
      snap={snap}
      onSnapChange={setSnap}
      footer={nav}
      backdrop={backdrop}
      onEdit={noop}
      onCopyName={noop}
      onCopyAddress={noop}
    />
  );
}

const sheet = (
  id: string,
  title: string,
  props: Parameters<typeof SheetPreview>[0],
) =>
  registerPreview(id, {
    title,
    orientation: 'portrait',
    render: () => <SheetPreview {...props} />,
  });

sheet('profile__profile__9-509', 'Profile: half', { snap: 'half' });
sheet('profile__profile__340-4509', 'Profile: half (current)', {
  snap: 'half',
});
sheet('profile__profile-fullview__49-1616', 'Profile: full view', {
  snap: 'full',
});
sheet('profile__profile__340-5133', 'Profile: no About me', {
  snap: 'half',
  profile: noAbout,
});
sheet('profile__profile__340-5575', 'Profile: peek', { snap: 'peek' });
sheet('profile__profile__270-10205', 'Profile: lavender backdrop', {
  snap: 'half',
  backdrop: 'lavender',
});

const landscape = (
  id: string,
  title: string,
  profile: Profile,
  props: { fullscreen?: boolean; initialExpanded?: boolean } = {},
) =>
  registerPreview(id, {
    title,
    orientation: 'landscape',
    render: () => (
      <ProfileLandscape
        profile={profile}
        onClose={noop}
        onEdit={noop}
        onCopyAddress={noop}
        {...props}
      />
    ),
  });

landscape(
  'profile__profile-landscape__232-2269',
  'Landscape card',
  MOCK_PROFILE,
);
landscape(
  'profile__profile-landscape__232-2806',
  'Landscape card: expanded',
  MOCK_PROFILE,
  {
    initialExpanded: true,
  },
);
landscape(
  'profile__profile-landscape__303-4252',
  'Landscape: data only',
  dataOnly(3),
);
landscape(
  'profile__profile-landscape__303-5189',
  'Landscape: data, see more',
  dataOnly(6),
);
landscape(
  'profile__profile-landscape__303-5658',
  'Landscape: data expanded',
  dataOnly(6),
  {
    initialExpanded: true,
  },
);
landscape('menu__profile__533-5757', 'Landscape full screen', MOCK_PROFILE, {
  fullscreen: true,
});

const plain: Profile = {
  ...noAbout,
  name: 'Carlos',
  hasClaimedName: true,
};
const tagged: Profile = { ...noAbout, name: '', hasClaimedName: false };
const claimed: Profile = { ...noAbout, name: 'Name', hasClaimedName: true };
const text = { description: 'text' };

const edit = (
  id: string,
  title: string,
  profile: Profile,
  props: Partial<EditProfileProps> = {},
) =>
  registerPreview(id, {
    title,
    orientation: 'portrait',
    render: () => (
      <EditProfile
        profile={profile}
        names={[]}
        onClose={noop}
        onSave={noop}
        {...props}
      />
    ),
  });

edit('profile__edit-profile__270-8413', 'Edit: plain username', plain, {
  initial: { draft: { ...text, country: 'Albania' } },
});
edit('profile__edit-profile__270-9413', 'Edit: leave dialog', plain, {
  initial: { draft: { ...text, country: 'Albania' }, dialog: 'leave' },
});
edit('profile__edit-profile__415-8560', 'Edit: non-claimed', tagged, {
  initial: { draft: { name: 'text', ...text } },
});
edit('profile__edit-profile__491-9606', 'Edit: non-claimed, value', tagged, {
  initial: { draft: { name: 'username', ...text } },
});
edit('profile__edit-profile__415-8733', 'Edit: link chip', tagged, {
  initial: {
    draft: {
      name: 'text',
      ...text,
      links: [{ title: 'Instagram', url: 'https://instagram.com/dcl' }],
    },
  },
});

const link = (
  id: string,
  title: string,
  linkInitial: EditProfileProps['initial'],
) =>
  edit(id, title, tagged, {
    initial: {
      draft: { name: 'text', ...text },
      dialog: 'link',
      ...linkInitial,
    },
  });
link('profile__addllink-modal__417-9081', 'Add link: empty', {});
link('profile__addllink-modal__417-11210', 'Add link: title', {
  link: { title: 'Instagram' },
  focus: 'linkTitle',
});
link('profile__addllink-modal__418-11432', 'Add link: URL focused', {
  focus: 'linkUrl',
});
link('profile__addllink-modal__418-11672', 'Add link: filled', {
  link: { title: 'Instagram', url: 'https://www.instagram.com/rubenmusic' },
});

const claimedEdit = (
  id: string,
  title: string,
  initial: EditProfileProps['initial'],
  profile: Profile = claimed,
) =>
  edit(id, title, profile, {
    names: MOCK_NAMES,
    initial: { draft: text, ...initial },
  });
claimedEdit('profile__edit-profile__491-8375', 'Edit: claimed, closed', {});
claimedEdit('profile__edit-profile__491-8558', 'Edit: names open', {
  open: 'username',
});
claimedEdit('profile__edit-profile__491-9803', 'Edit: names open, Name 02', {
  open: 'username',
  picked: 'Name 02',
});
claimedEdit('profile__edit-profile__491-10036', 'Edit: Name 02 picked', {
  picked: 'Name 02',
});
claimedEdit('profile__edit-profile__491-8739', 'Edit: non-unique, empty', {
  tab: 'nonUnique',
  draft: { name: '', ...text },
});
claimedEdit('profile__edit-profile__491-8960', 'Edit: non-unique, typing', {
  tab: 'nonUnique',
  draft: { name: 'Other Name', ...text },
});
claimedEdit('profile__edit-profile__491-9149', 'Edit: non-unique, filled', {
  tab: 'nonUnique',
  draft: { name: 'Other Name', ...text },
});

const guest: Profile = { ...plain, guest: true };
edit(
  'profile__edit-profile__86-2417',
  'Edit: guest, username disabled',
  guest,
  {
    initial: { draft: { ...text, country: 'Albania' } },
  },
);
edit('profile__edit-profile__242-2171', 'Edit: full form', guest, {
  initial: { draft: text },
});
edit('profile__edit-profile__144-2750', 'Edit: keyboard', guest, {
  initial: { draft: text, focus: 'description' },
});
edit('profile__edit-profile__144-2195', 'Edit: keyboard, scrolled', guest, {
  initial: { draft: text, focus: 'description', scrollY: 128 },
});
edit('profile__edit-profile__144-2419', 'Edit: keyboard, country list', guest, {
  initial: { draft: text, focus: 'description', open: 'country', scrollY: 360 },
});
edit('profile__edit-profile__54-2414', 'Edit: list opens up', guest, {
  initial: { draft: text, open: 'gender', scrollY: 230 },
});
edit('profile__edit-profile__86-2087', 'Edit: list opens down', guest, {
  initial: { draft: text, open: 'country', scrollY: 360 },
});

const fragment = (
  id: string,
  title: string,
  label: string,
  options: OptionId[],
) =>
  registerPreview(id, {
    title,
    orientation: 'landscape',
    render: () => (
      <View style={styles.fragment}>
        <Dropdown
          label={label}
          value={null}
          options={options
            .slice(0, 5)
            .map(o => ({ value: o.id, label: t(o.label) }))}
          onChange={noop}
          defaultOpen
        />
      </View>
    ),
  });
fragment(
  'profile__dropdown__244-2980',
  'Dropdown: pronouns',
  t('PROFILE_PRONOUNS'),
  PRONOUNS,
);
fragment(
  'profile__dropdown__244-3045',
  'Dropdown: gender',
  t('PROFILE_GENDER'),
  GENDERS,
);
fragment(
  'profile__dropdown__244-3076',
  'Dropdown: orientation',
  t('PROFILE_SEXUAL_ORIENTATION'),
  ORIENTATIONS,
);
fragment(
  'profile__dropdown__254-3614',
  'Dropdown: relationship',
  t('PROFILE_RELATIONSHIP'),
  RELATIONSHIPS,
);

const styles = StyleSheet.create({
  fragment: { flex: 1, padding: u(24), backgroundColor: colors.purple450 },
});
