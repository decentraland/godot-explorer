import { useEffect, useRef, useState } from 'react';
import {
  BackHandler,
  Image,
  Keyboard,
  Pressable,
  ScrollView,
  StyleSheet,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, Dialog, Icon, LinearFill, TextField } from '../ds';
import { u } from '../ds/scale';
import type { I18nKey } from '../i18n';
import { t } from '../i18n';
import { AddLinkDialog } from './AddLinkDialog';
import { COUNTRIES } from './countries';
import { EditFooter } from './EditFooter';
import { FieldDropdown } from './FieldDropdown';
import { LinkChips } from './LinkChips';
import {
  buildPatch,
  draftFromProfile,
  isDirty,
  LIMITS,
  MAX_LINKS,
  resolveUsername,
} from './logic';
import {
  GENDERS,
  ORIENTATIONS,
  OptionId,
  PRONOUNS,
  RELATIONSHIPS,
} from './options';
import type {
  EditDraft,
  Profile,
  ProfileDeployPayload,
  ProfileLink,
  UsernameTab,
} from './types';
import { UsernameField, usernameError, usernameVariant } from './UsernameField';

type DropdownKey =
  | 'username'
  | 'country'
  | 'pronouns'
  | 'gender'
  | 'sexualOrientation'
  | 'relationshipStatus';

export type EditProfileProps = {
  profile: Profile;
  /** Minted names of the account; empty until loaded or when it owns none. */
  names: readonly string[];
  onClose: () => void;
  onSave: (patch: ProfileDeployPayload) => void;
  /** Starting UI state, for previews. */
  initial?: {
    draft?: Partial<EditDraft>;
    tab?: UsernameTab;
    picked?: string;
    open?: DropdownKey;
    dialog?: 'leave' | 'link';
    link?: Partial<ProfileLink>;
    focus?: 'description' | 'linkTitle' | 'linkUrl';
    scrollY?: number;
  };
};

const optionList = (options: OptionId[]) =>
  options.map(o => ({ value: o.id, label: t(o.label) }));

const countryOptions = COUNTRIES.map(c => ({ value: c, label: c }));

const TEXT_FIELDS = [
  ['employmentStatus', 'PROFILE_EMPLOYMENT_STATUS'],
  ['profession', 'PROFILE_PROFESSION'],
  ['realName', 'PROFILE_REAL_NAME'],
  ['hobbies', 'PROFILE_HOBBY'],
] as const satisfies readonly (readonly [keyof EditDraft, I18nKey])[];

function useKeyboardVisible(): boolean {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const show = Keyboard.addListener('keyboardDidShow', () =>
      setVisible(true),
    );
    const hide = Keyboard.addListener('keyboardDidHide', () =>
      setVisible(false),
    );
    return () => {
      show.remove();
      hide.remove();
    };
  }, []);
  return visible;
}

/** Edit Profile form: dirty tracking, leave guard, Add Link dialog and a patch of the changed fields on SAVE. */
export function EditProfile({
  profile,
  names,
  onClose,
  onSave,
  initial,
}: EditProfileProps) {
  const insets = useSafeAreaInsets();
  const keyboard = useKeyboardVisible();
  const scroller = useRef<ScrollView>(null);
  const [draft, setDraft] = useState<EditDraft>({
    ...draftFromProfile(profile),
    ...initial?.draft,
  });
  const [tab, setTab] = useState<UsernameTab>(
    initial?.tab ?? (profile.hasClaimedName ? 'unique' : 'nonUnique'),
  );
  const [picked, setPicked] = useState(
    initial?.picked ??
      (names.includes(profile.name) ? profile.name : names[0] ?? ''),
  );
  const [dialog, setDialog] = useState(initial?.dialog);
  const [open, setOpen] = useState<DropdownKey | undefined>(initial?.open);
  const [scrollY, setScrollY] = useState(initial?.scrollY ?? 0);
  const [viewport, setViewport] = useState(0);

  const set = <K extends keyof EditDraft>(key: K, value: EditDraft[K]) =>
    setDraft(current => ({ ...current, [key]: value }));
  const variant = usernameVariant(names, profile.hasClaimedName);
  const patch = buildPatch(
    profile,
    draft,
    resolveUsername(tab, draft.name, names, picked),
  );
  const typing = variant !== 'tabs' || tab === 'nonUnique';
  const canSave =
    isDirty(patch) && !(typing && !profile.guest && usernameError(draft.name));
  const dirty = isDirty(patch);

  const leave = () => (dirty ? setDialog('leave') : onClose());
  const guard = useRef(leave);
  guard.current = leave;
  useEffect(() => {
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      guard.current();
      return true;
    });
    return () => sub.remove();
  }, []);

  const dropdown = (
    key: Exclude<DropdownKey, 'username'>,
    label: I18nKey,
    options: { value: string; label: string }[],
  ) => (
    <FieldDropdown
      label={t(label)}
      value={draft[key] || null}
      options={options}
      onChange={value => set(key, value)}
      scrollY={scrollY}
      visibleBottom={viewport}
      open={open === key}
      onOpenChange={isOpen => setOpen(isOpen ? key : undefined)}
    />
  );

  return (
    <View style={styles.root}>
      <LinearFill
        stops={[
          { offset: 0, color: '#7D26B5' },
          { offset: 0.3, color: '#470071' },
          { offset: 1, color: '#470071' },
        ]}
        angle={20}
      />
      <View style={[styles.header, { paddingTop: insets.top + u(10) }]}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('PROFILE_BACK')}
          hitSlop={12}
          onPress={leave}
        >
          <Icon name="chevronLeft" size={26} />
        </Pressable>
        <AppText variant="heading" style={styles.title}>
          {t('PROFILE_EDIT_PROFILE')}
        </AppText>
      </View>
      <ScrollView
        ref={scroller}
        style={styles.flex}
        contentContainerStyle={styles.content}
        keyboardShouldPersistTaps="handled"
        scrollEventThrottle={100}
        onScroll={e => setScrollY(e.nativeEvent.contentOffset.y)}
        onLayout={e => {
          setViewport(e.nativeEvent.layout.height);
          if (initial?.scrollY) {
            scroller.current?.scrollTo({ y: initial.scrollY, animated: false });
          }
        }}
      >
        <View style={styles.avatar}>
          {profile.faceUrl ? (
            <Image source={{ uri: profile.faceUrl }} style={styles.face} />
          ) : (
            <Icon name="user" size={48} color={colors.purple350} />
          )}
        </View>
        <View style={open === 'username' ? styles.raised : undefined}>
          <UsernameField
            variant={variant}
            value={draft.name}
            onChange={name => set('name', name)}
            tag={profile.tag}
            names={names}
            tab={tab}
            onTab={setTab}
            picked={picked}
            onPick={setPicked}
            disabled={profile.guest}
            defaultOpen={open === 'username'}
            onOpenChange={isOpen => setOpen(isOpen ? 'username' : undefined)}
          />
        </View>
        <TextField
          label={t('PROFILE_DESCRIPTION')}
          helper={t('PROFILE_DESCRIBE_YOURSELF_IN_A_FEW_WORDS')}
          helperAbove
          autoFocus={initial?.focus === 'description'}
          value={draft.description}
          onChangeText={text => set('description', text)}
          multiline
          multilineHeight={108}
          maxLength={LIMITS.description}
          showCounter
          counterBelow
        />
        <View>
          <AppText variant="body" style={styles.label}>
            {t('PROFILE_LINKS')}
          </AppText>
          <AppText variant="caption" style={styles.helper}>
            {t('PROFILE_ADD_UP_TO_5_LINKS_TO')}
          </AppText>
          <LinkChips
            links={draft.links}
            onRemove={index =>
              set(
                'links',
                draft.links.filter((_, i) => i !== index),
              )
            }
          />
          {draft.links.length < MAX_LINKS && (
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={t('PROFILE_ADD_LINK_CAPS')}
              onPress={() => setDialog('link')}
              style={[styles.add, draft.links.length > 0 && styles.addGap]}
            >
              <Icon name="plus" size={10} strokeWidth={2} />
              <AppText variant="button" style={styles.addLabel}>
                {t('PROFILE_ADD_LINK_CAPS')}
              </AppText>
            </Pressable>
          )}
        </View>
        {dropdown('country', 'PROFILE_COUNTRY', countryOptions)}
        {dropdown('pronouns', 'PROFILE_PRONOUNS', optionList(PRONOUNS))}
        {dropdown('gender', 'PROFILE_GENDER', optionList(GENDERS))}
        {dropdown(
          'sexualOrientation',
          'PROFILE_SEXUAL_ORIENTATION',
          optionList(ORIENTATIONS),
        )}
        {dropdown(
          'relationshipStatus',
          'PROFILE_RELATIONSHIP',
          optionList(RELATIONSHIPS),
        )}
        {TEXT_FIELDS.map(([key, label]) => (
          <TextField
            key={key}
            label={t(label)}
            value={draft[key]}
            onChangeText={text => set(key, text)}
            onClear={() => set(key, '')}
            maxLength={LIMITS.field}
          />
        ))}
      </ScrollView>
      {!keyboard && (
        <EditFooter
          canSave={canSave}
          bottomInset={insets.bottom}
          onCancel={leave}
          onSave={() => onSave(patch)}
        />
      )}
      <Dialog
        visible={dialog === 'leave'}
        icon="warning"
        rounded
        width={260}
        title={t('PROFILE_LEAVE_TITLE')}
        body={t('PROFILE_LEAVE_BODY')}
        onRequestClose={() => setDialog(undefined)}
        buttons={[
          { label: t('PROFILE_LEAVE'), tone: 'dark', onPress: onClose },
          { label: t('PROFILE_STAY'), onPress: () => setDialog(undefined) },
        ]}
      />
      <AddLinkDialog
        visible={dialog === 'link'}
        initial={initial?.link}
        focus={
          initial?.focus === 'linkTitle'
            ? 'title'
            : initial?.focus === 'linkUrl'
            ? 'url'
            : undefined
        }
        onCancel={() => setDialog(undefined)}
        onSave={link => {
          set('links', [...draft.links, link]);
          setDialog(undefined);
        }}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  flex: { flex: 1 },
  raised: { zIndex: 50 },
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple450 },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(12),
    paddingHorizontal: u(24),
    paddingBottom: u(8),
  },
  title: { fontSize: u(24) },
  content: { paddingHorizontal: u(24), paddingVertical: u(16), gap: u(30) },
  avatar: {
    alignSelf: 'center',
    width: u(88),
    height: u(88),
    borderRadius: u(44),
    borderWidth: u(2),
    borderColor: colors.ruby,
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
    backgroundColor: colors.lavender,
  },
  face: { width: '100%', height: '100%' },
  label: { fontSize: u(15), fontFamily: 'Inter-Medium', color: colors.snow },
  helper: { fontSize: u(10), color: colors.pearl, marginVertical: u(5) },
  add: {
    alignSelf: 'flex-start',
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(6),
    height: u(34),
    paddingHorizontal: u(12),
    marginTop: u(5),
    borderRadius: u(8),
    backgroundColor: colors.black40,
  },
  addGap: { marginTop: u(12) },
  addLabel: { fontFamily: 'Inter-Bold', fontSize: u(11) },
});
