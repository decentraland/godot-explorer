import { ReactNode, useEffect, useState } from 'react';
import {
  ActivityIndicator,
  Image,
  KeyboardAvoidingView,
  Linking,
  Platform,
  Pressable,
  StyleSheet,
  TextInput,
  View,
} from 'react-native';
import { AppText, colors, PrimaryButton, radii } from '../ds';
import { FlowFrame } from './FlowFrame';
import { isValidName, Provider } from './flowMachine';
import { I18nKey, t } from '../i18n';
import { u } from '../ds/scale';

const logo = require('../assets/logo.png');

const TERMS_URL = 'https://decentraland.org/terms';

export function SplashScreen({ onDone }: { onDone: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onDone, 1200);
    return () => clearTimeout(timer);
  }, [onDone]);
  return (
    <FlowFrame flat>
      <View style={styles.center}>
        <Image source={logo} style={styles.logo} resizeMode="contain" />
      </View>
    </FlowFrame>
  );
}

export function TermsScreen({ onAccept }: { onAccept: () => void }) {
  return (
    <FlowFrame>
      <View style={styles.center}>
        <Image source={logo} style={styles.logoSmall} resizeMode="contain" />
        <AppText variant="heading" style={styles.title}>
          {t('FLOW_TERMS_TITLE')}
        </AppText>
        <AppText variant="body" style={styles.copy}>
          {t('FLOW_TERMS_BODY')}
        </AppText>
        <Pressable
          accessibilityRole="link"
          onPress={() => Linking.openURL(TERMS_URL)}
          hitSlop={8}
        >
          <AppText variant="button" style={styles.link}>
            {t('FLOW_TERMS_READ')}
          </AppText>
        </Pressable>
      </View>
      <PrimaryButton label={t('FLOW_TERMS_ACCEPT')} onPress={onAccept} />
    </FlowFrame>
  );
}

export function AccountScreen({
  onGuest,
  onSignIn,
  onBack,
}: {
  onGuest: () => void;
  onSignIn: () => void;
  onBack: () => void;
}) {
  return (
    <FlowFrame onBack={onBack}>
      <View style={styles.accountTop} />
      <View style={styles.brand}>
        <Image source={logo} style={styles.brandMark} resizeMode="contain" />
        <AppText variant="title" style={styles.brandName}>
          {t('FLOW_BRAND')}
        </AppText>
      </View>
      <AppText variant="heading" style={styles.accountTitle}>
        {t('FLOW_ACCOUNT_TITLE')}
      </AppText>
      <AppText variant="body" style={styles.accountBody}>
        <AppText variant="body" style={styles.accountBodyBold}>
          {t('FLOW_ACCOUNT_BODY_BOLD')}
        </AppText>
        {t('FLOW_ACCOUNT_BODY_REST')}
      </AppText>
      <View style={styles.accountActions}>
        <PrimaryButton
          rounded
          label={t('FLOW_ACCOUNT_CREATE')}
          onPress={onSignIn}
        />
        <PrimaryButton
          rounded
          tone="dark"
          label={t('FLOW_ACCOUNT_SIGN_IN')}
          onPress={onSignIn}
        />
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('FLOW_ACCOUNT_GUEST')}
          onPress={onGuest}
          hitSlop={8}
          style={styles.guest}
        >
          <AppText variant="caption" style={styles.guestLabel}>
            {t('FLOW_ACCOUNT_GUEST')}
          </AppText>
        </Pressable>
      </View>
      <View style={styles.accountBottom} />
    </FlowFrame>
  );
}

const PROVIDERS: { key: Provider; label: I18nKey }[] = [
  { key: 'google', label: 'FLOW_PROVIDER_GOOGLE' },
  { key: 'apple', label: 'FLOW_PROVIDER_APPLE' },
  { key: 'email', label: 'FLOW_PROVIDER_EMAIL' },
  { key: 'wallet', label: 'FLOW_PROVIDER_WALLET' },
];

export function SignInScreen({
  error,
  onProvider,
  onBack,
}: {
  error: string | null;
  onProvider: (provider: Provider) => void;
  onBack: () => void;
}) {
  return (
    <FlowFrame onBack={onBack}>
      <View style={styles.center}>
        <AppText variant="heading" style={styles.title}>
          {t('FLOW_SIGN_IN_TITLE')}
        </AppText>
        <AppText variant="body" style={styles.copy}>
          {t('FLOW_SIGN_IN_BODY')}
        </AppText>
        {error ? (
          <AppText variant="caption" style={styles.error}>
            {error}
          </AppText>
        ) : null}
      </View>
      <View style={styles.actions}>
        {PROVIDERS.map(provider => (
          <PrimaryButton
            key={provider.key}
            label={t(provider.label)}
            tone="secondary"
            onPress={() => onProvider(provider.key)}
          />
        ))}
      </View>
    </FlowFrame>
  );
}

export function WaitScreen({
  title,
  body,
  onCancel,
}: {
  title: string;
  body: string;
  onCancel?: () => void;
}) {
  return (
    <FlowFrame>
      <View style={styles.center}>
        <ActivityIndicator size="large" color={colors.snow} />
        <AppText variant="heading" style={styles.title}>
          {title}
        </AppText>
        <AppText variant="body" style={styles.copy}>
          {body}
        </AppText>
      </View>
      {onCancel ? (
        <PrimaryButton
          label={t('FLOW_CANCEL')}
          tone="ghost"
          onPress={onCancel}
        />
      ) : null}
    </FlowFrame>
  );
}

export function NamingScreen({
  initial,
  preview,
  onSubmit,
  onBack,
}: {
  initial: string;
  preview: ReactNode;
  onSubmit: (name: string) => void;
  onBack: () => void;
}) {
  const [name, setName] = useState(initial);
  const valid = isValidName(name);
  return (
    <KeyboardAvoidingView
      style={styles.fill}
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
    >
      <FlowFrame onBack={onBack}>
        <AppText variant="heading" style={styles.titleLeft}>
          {t('FLOW_NAME_TITLE')}
        </AppText>
        {preview}
        <TextInput
          value={name}
          onChangeText={setName}
          placeholder={t('FLOW_NAME_PLACEHOLDER')}
          placeholderTextColor={colors.silver}
          autoCapitalize="none"
          autoCorrect={false}
          maxLength={15}
          style={styles.input}
        />
        <AppText
          variant="caption"
          style={[styles.hint, !valid && name.length > 0 && styles.error]}
        >
          {t('FLOW_NAME_HINT')}
        </AppText>
        <PrimaryButton
          label={t('FLOW_LETS_GO')}
          disabled={!valid}
          onPress={() => onSubmit(name.trim())}
        />
      </FlowFrame>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  center: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  logo: { width: u(160), height: u(160) },
  logoSmall: { width: u(96), height: u(96), marginBottom: u(12) },
  accountTop: { flex: 3 },
  accountBottom: { flex: 2 },
  brand: { flexDirection: 'row', alignItems: 'center', gap: u(6) },
  brandMark: { width: u(18), height: u(18) },
  brandName: { fontFamily: 'Inter-Bold', fontSize: u(16) },
  accountTitle: { fontSize: u(36), marginTop: u(12) },
  accountBody: { fontSize: u(18), lineHeight: u(26), marginTop: u(8) },
  accountBodyBold: { fontFamily: 'Inter-Bold', fontSize: u(18) },
  accountActions: { gap: u(12), marginTop: u(60) },
  guest: { alignSelf: 'center', paddingVertical: u(8) },
  guestLabel: { color: colors.snow, textDecorationLine: 'underline' },
  title: { marginTop: u(16), textAlign: 'center' },
  titleLeft: { marginTop: u(4), marginBottom: u(8) },
  copy: {
    marginTop: u(8),
    textAlign: 'center',
    opacity: 0.85,
    paddingHorizontal: u(16),
  },
  link: { marginTop: u(14), color: colors.lavender },
  error: { marginTop: u(12), color: colors.ruby },
  actions: { gap: u(12) },
  input: {
    height: u(52),
    borderRadius: radii.pill,
    paddingHorizontal: u(20),
    color: colors.snow,
    fontFamily: 'Inter-Medium',
    fontSize: u(16),
    backgroundColor: colors.white10,
    borderWidth: 1,
    borderColor: colors.white30,
  },
  hint: { marginTop: u(8), marginBottom: u(16), textAlign: 'center' },
});
