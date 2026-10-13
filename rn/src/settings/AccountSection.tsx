import { useState } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import {
  AppText,
  colors,
  Dialog,
  fonts,
  OutlinedButton,
  ReadOnlyField,
  u,
} from '../ds';
import { t } from '../i18n';
import { shortenAddress } from './format';
import type { SettingsModel } from './useSettings';

type DialogKind = 'signOut' | 'delete' | 'deleteFailed' | null;

const DELETE_LINK = '#FF5F77';

function GuestCard({ onAddEmail }: { onAddEmail: () => void }) {
  return (
    <View style={styles.card}>
      <AppText variant="title" style={styles.cardTitle}>
        {t('SETTINGS_UNLOCK_TITLE')}
      </AppText>
      <AppText variant="body" style={styles.cardBody}>
        {t('SETTINGS_UNLOCK_BODY')}
      </AppText>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('SETTINGS_ADD_EMAIL')}
        hitSlop={8}
        onPress={onAddEmail}
        style={styles.addEmail}
      >
        <AppText variant="button" style={styles.addEmailText}>
          {t('SETTINGS_ADD_EMAIL')}
        </AppText>
      </Pressable>
    </View>
  );
}

/** Identity, guest upgrade or email, then sign out and delete pinned to the bottom of the pane. */
export function AccountSection({
  m,
  onSignedOut,
  initialDialog = null,
}: {
  m: SettingsModel;
  onSignedOut?: () => void;
  initialDialog?: DialogKind;
}) {
  const { account, api } = m;
  const [dialog, setDialog] = useState<DialogKind>(initialDialog);

  const signOut = async () => {
    setDialog(null);
    await api.signOut();
    onSignedOut?.();
  };
  const deleteAccount = async () => {
    setDialog(null);
    const result = await api.deleteAccount();
    if (result.ok) {
      onSignedOut?.();
    } else {
      setDialog('deleteFailed');
    }
  };
  const cancel = { label: t('SETTINGS_CANCEL'), tone: 'secondary' as const };

  return (
    <View style={styles.root}>
      <View style={styles.top}>
        <View style={styles.identity}>
          <AppText variant="heading" numberOfLines={1} style={styles.name}>
            {account.name}
          </AppText>
          {!!account.tag && (
            <AppText variant="body" style={styles.tag}>
              {account.tag}
            </AppText>
          )}
        </View>
        <AppText variant="caption" style={styles.address}>
          {shortenAddress(account.address)}
        </AppText>
        <View style={styles.gap}>
          {account.guest ? (
            <GuestCard
              onAddEmail={() => {
                api.startEmailUpgrade();
              }}
            />
          ) : (
            <ReadOnlyField
              label={t('SETTINGS_EMAIL')}
              value={account.email ?? ''}
            />
          )}
        </View>
      </View>
      <View style={styles.bottom}>
        <OutlinedButton
          box
          icon="logout"
          label={t('CUSTOM_BUTTON_SIGN_OUT')}
          onPress={() => setDialog('signOut')}
        />
        <Pressable
          accessibilityRole="link"
          accessibilityLabel={t('SETTINGS_DELETE_LINK')}
          hitSlop={8}
          onPress={() => setDialog('delete')}
        >
          <AppText variant="body" style={styles.delete}>
            {t('SETTINGS_DELETE_PROMPT')}
            <Text style={styles.deleteLink}>{t('SETTINGS_DELETE_LINK')}</Text>
          </AppText>
        </Pressable>
      </View>
      <Dialog
        visible={dialog === 'signOut'}
        title={t('SETTINGS_SIGN_OUT_TITLE')}
        body={t('SETTINGS_SIGN_OUT_BODY')}
        onRequestClose={() => setDialog(null)}
        buttons={[
          { ...cancel, onPress: () => setDialog(null) },
          {
            label: t('CUSTOM_BUTTON_SIGN_OUT'),
            onPress: () => {
              signOut();
            },
          },
        ]}
      />
      <Dialog
        visible={dialog === 'delete'}
        icon="warning"
        title={t('SETTINGS_DELETE_TITLE')}
        body={t('SETTINGS_DELETE_BODY')}
        onRequestClose={() => setDialog(null)}
        buttons={[
          { ...cancel, onPress: () => setDialog(null) },
          {
            label: t('SETTINGS_DELETE_CONFIRM'),
            onPress: () => {
              deleteAccount();
            },
          },
        ]}
      />
      <Dialog
        visible={dialog === 'deleteFailed'}
        title={t('SETTINGS_DELETE_FAILED_TITLE')}
        body={t('SETTINGS_DELETE_FAILED_BODY')}
        onRequestClose={() => setDialog(null)}
        buttons={[{ label: t('SETTINGS_OK'), onPress: () => setDialog(null) }]}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, justifyContent: 'space-between' },
  top: { marginTop: -u(4) },
  identity: {
    flexDirection: 'row',
    alignItems: 'baseline',
    justifyContent: 'space-between',
  },
  name: { flex: 1, fontSize: u(18) },
  tag: { fontSize: u(14), color: colors.lavender70, marginLeft: u(8) },
  address: {
    fontFamily: fonts.bold,
    fontSize: u(12),
    color: colors.lavender,
    marginTop: u(4),
  },
  gap: { marginTop: u(26) },
  card: {
    padding: u(13),
    borderRadius: u(5),
    borderWidth: 1,
    borderColor: colors.lavender,
    backgroundColor: colors.black40,
  },
  cardTitle: { fontFamily: fonts.bold, fontSize: u(16), color: colors.snow },
  cardBody: { fontSize: u(13), color: colors.snow, marginTop: u(4) },
  addEmail: {
    height: u(27),
    marginTop: u(12),
    borderRadius: u(3),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.ruby,
  },
  addEmailText: { fontFamily: fonts.bold, fontSize: u(11) },
  bottom: { gap: u(32), paddingTop: u(32) },
  delete: { textAlign: 'center', fontSize: u(13), color: colors.snow },
  deleteLink: { color: DELETE_LINK, textDecorationLine: 'underline' },
});
