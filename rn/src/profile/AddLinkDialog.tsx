import { useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { Dialog, TextField } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { canSaveLink, LIMITS, validateText } from './logic';
import type { ProfileLink } from './types';

/** "Add Link" modal; SAVE enables once the title is set and the URL is a valid http(s) address. */
export function AddLinkDialog({
  visible,
  onCancel,
  onSave,
  initial,
  focus,
}: {
  visible: boolean;
  onCancel: () => void;
  onSave: (link: ProfileLink) => void;
  initial?: Partial<ProfileLink>;
  /** Field that starts focused, for previews. */
  focus?: 'title' | 'url';
}) {
  // Mounted only while visible so every opening starts empty.
  return visible ? (
    <Form onCancel={onCancel} onSave={onSave} initial={initial} focus={focus} />
  ) : null;
}

function Form({
  onCancel,
  onSave,
  initial,
  focus,
}: {
  onCancel: () => void;
  onSave: (link: ProfileLink) => void;
  initial?: Partial<ProfileLink>;
  focus?: 'title' | 'url';
}) {
  const [title, setTitle] = useState(initial?.title ?? '');
  const [url, setUrl] = useState(initial?.url ?? '');
  const valid = canSaveLink(title, url);
  const urlInvalid = validateText(url, Number.MAX_SAFE_INTEGER, { url: true });
  return (
    <Dialog
      visible
      rounded
      width={310}
      title={t('PROFILE_ADD_LINK')}
      onRequestClose={onCancel}
      buttons={[
        { label: t('COMMON_CANCEL'), tone: 'dark', onPress: onCancel },
        {
          label: t('PROFILE_SAVE'),
          disabled: !valid,
          onPress: () => onSave({ title: title.trim(), url }),
        },
      ]}
    >
      <View style={styles.fields}>
        <TextField
          label={t('PROFILE_LINK_TITLE')}
          value={title}
          onChangeText={setTitle}
          onClear={() => setTitle('')}
          placeholder={t('PROFILE_E_G_INSTAGRAM')}
          maxLength={LIMITS.linkTitle}
          showCounter
          autoFocus={focus === 'title'}
          counterBelow
        />
        <TextField
          label={t('PROFILE_LINK')}
          value={url}
          onChangeText={setUrl}
          onClear={() => setUrl('')}
          placeholder={t('PROFILE_LINK_URL_HTTPS')}
          autoFocus={focus === 'url'}
          keyboardType="url"
          autoCapitalize="none"
          error={urlInvalid ? t('INPUTS_ENTER_VALID_URL') : undefined}
        />
      </View>
    </Dialog>
  );
}

const styles = StyleSheet.create({ fields: { marginTop: u(14), gap: u(14) } });
