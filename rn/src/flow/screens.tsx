import { ReactNode, useEffect, useState } from 'react';
import {
  ActivityIndicator,
  Image,
  KeyboardAvoidingView,
  Linking,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { core } from '../core';
import {
  AppText,
  colors,
  Button,
  LoginButton,
  Spinner,
  u,
} from '@dcl/ui-native';
import { FlowFrame } from './FlowFrame';
import { isValidName, Provider } from './flowMachine';
import { I18nKey, t } from '../i18n';

const logo = require('../assets/logo.png');
const decentralandLogo = require('../assets/decentraland_logo.png');

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

/** Inline "Terms of Service, Privacy Policy, and Content Policy" disclaimer with real links,
 * shared by Account Home and Sign In. */
function TermsDisclaimer() {
  const urls = core().urls();
  const links: Record<string, { label: string; url: string }> = {
    tos: { label: t('FLOW_TERMS_OF_SERVICE'), url: urls.termsOfService },
    privacy: { label: t('FLOW_PRIVACY_POLICY'), url: urls.privacyPolicy },
    content: { label: t('FLOW_CONTENT_POLICY'), url: urls.contentPolicy },
  };
  return (
    <AppText variant="body" style={styles.disclaimer}>
      {t('FLOW_TERMS_DISCLAIMER')
        .split(/(\{\w+\})/g)
        .map((part, i) => {
          const link = links[part.slice(1, -1)];
          return link ? (
            <Text
              key={i}
              style={styles.disclaimerLink}
              onPress={() => Linking.openURL(link.url)}
            >
              {link.label}
            </Text>
          ) : (
            part
          );
        })}
    </AppText>
  );
}

export function AccountScreen({
  onGuest,
  onSignIn,
}: {
  onGuest: () => void;
  onSignIn: () => void;
}) {
  return (
    <FlowFrame photo edgeToEdge>
      <View style={styles.accountContainer}>
        <View>
          <Image
            source={decentralandLogo}
            style={styles.brandMark}
            resizeMode="contain"
          />
          <AppText variant="heading" style={styles.flowTitle}>
            {t('FLOW_ACCOUNT_TITLE')}
          </AppText>
          <AppText variant="body" style={styles.accountBody}>
            {t('FLOW_ACCOUNT_TAGLINE')}
          </AppText>
        </View>
        <View style={styles.accountActions}>
          <View style={styles.accountButtons}>
            <Button
              big
              fullWidth
              label={t('FLOW_ACCOUNT_GUEST')}
              onPress={onGuest}
            />
            <Button
              big
              fullWidth
              tone="outline"
              label={t('FLOW_ACCOUNT_SIGN_IN')}
              onPress={onSignIn}
            />
          </View>
          <TermsDisclaimer />
        </View>
      </View>
    </FlowFrame>
  );
}

const BRAND_PROVIDERS: { key: Provider; label: I18nKey }[] = [
  { key: 'google', label: 'FLOW_PROVIDER_GOOGLE' },
  { key: 'wallet', label: 'FLOW_PROVIDER_WALLET' },
  { key: 'apple', label: 'FLOW_PROVIDER_APPLE' },
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
    <FlowFrame onBack={onBack} edgeToEdge>
      <ScrollView
        contentContainerStyle={styles.signInScroll}
        showsVerticalScrollIndicator={false}
      >
        <AppText variant="heading" style={styles.flowTitle}>
          {t('FLOW_SIGN_IN_TITLE')}
        </AppText>
        {error ? (
          <AppText variant="caption" style={styles.error}>
            {error}
          </AppText>
        ) : null}
        <View style={styles.actions}>
          {BRAND_PROVIDERS.map(provider => (
            <LoginButton
              key={provider.key}
              provider={provider.key}
              label={t(provider.label)}
              onPress={() => onProvider(provider.key)}
            />
          ))}
          <View style={styles.dividerRow}>
            <View style={styles.dividerLine} />
            <AppText variant="caption" style={styles.dividerLabel}>
              {t('FLOW_OR')}
            </AppText>
            <View style={styles.dividerLine} />
          </View>
          <LoginButton
            provider="email"
            label={t('FLOW_PROVIDER_EMAIL')}
            onPress={() => onProvider('email')}
          />
          <TermsDisclaimer />
        </View>
      </ScrollView>
    </FlowFrame>
  );
}

/** Figma `ACCOUNT_GUEST_CREATE` (1800:9275): the guest account being created over the Account
 * Home photo. */
export function GuestCreateScreen() {
  return (
    <FlowFrame photo edgeToEdge>
      <View style={styles.guestCreate}>
        <Spinner size={75} accessibilityLabel={t('FLOW_GUEST_WAIT_TITLE')} />
        <AppText variant="heading" style={styles.guestCreateTitle}>
          {t('FLOW_GUEST_WAIT_TITLE')}
        </AppText>
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
        <Button
          label={t('FLOW_CANCEL')}
          tone="ghost"
          fullWidth
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
    <FlowFrame onBack={onBack}>
      <KeyboardAvoidingView
        style={styles.fill}
        behavior="padding"
        keyboardVerticalOffset={u(48)}
      >
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
        <Button
          label={t('FLOW_LETS_GO')}
          disabled={!valid}
          fullWidth
          onPress={() => onSubmit(name.trim())}
        />
      </KeyboardAvoidingView>
    </FlowFrame>
  );
}

/** One text field and a button: the email address, then the code it was sent. */
function TextStep({
  title,
  body,
  placeholder,
  action,
  error,
  initial = '',
  keyboardType,
  maxLength,
  busy,
  valid,
  onSubmit,
  onBack,
}: {
  title: string;
  body: string;
  placeholder: string;
  action: string;
  error: string | null;
  initial?: string;
  keyboardType: 'email-address' | 'number-pad';
  maxLength?: number;
  busy: boolean;
  valid: (value: string) => boolean;
  onSubmit: (value: string) => void;
  onBack: () => void;
}) {
  const [value, setValue] = useState(initial);
  const [focused, setFocused] = useState(false);
  return (
    <FlowFrame onBack={onBack}>
      <KeyboardAvoidingView
        style={styles.fill}
        behavior="padding"
        keyboardVerticalOffset={u(48)}
      >
        <View style={styles.center}>
          <AppText variant="heading" style={styles.title}>
            {title}
          </AppText>
          <AppText variant="body" style={styles.copy}>
            {body}
          </AppText>
          {error ? (
            <AppText variant="caption" style={styles.error}>
              {error}
            </AppText>
          ) : null}
        </View>
        <TextInput
          value={value}
          onChangeText={setValue}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          placeholder={placeholder}
          placeholderTextColor={colors.white50}
          autoCapitalize="none"
          autoCorrect={false}
          autoFocus
          keyboardType={keyboardType}
          maxLength={maxLength}
          editable={!busy}
          style={[
            styles.input,
            styles.inputGap,
            error ? styles.inputError : focused && styles.inputFocused,
          ]}
        />
        <Button
          label={action}
          disabled={busy || !valid(value)}
          fullWidth
          onPress={() => onSubmit(value.trim())}
        />
      </KeyboardAvoidingView>
    </FlowFrame>
  );
}

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
export const isValidEmail = (email: string) => EMAIL_PATTERN.test(email.trim());
const CODE_PATTERN = /^\d{4,8}$/;
export const isValidCode = (code: string) => CODE_PATTERN.test(code.trim());

export function EmailScreen(props: {
  initial: string;
  error: string | null;
  busy: boolean;
  onSubmit: (email: string) => void;
  onBack: () => void;
}) {
  return (
    <TextStep
      {...props}
      title={t('FLOW_EMAIL_TITLE')}
      body={t('FLOW_EMAIL_BODY')}
      placeholder={t('FLOW_EMAIL_PLACEHOLDER')}
      action={t('FLOW_EMAIL_SEND')}
      keyboardType="email-address"
      valid={isValidEmail}
    />
  );
}

export function CodeScreen({
  email,
  ...props
}: {
  email: string;
  error: string | null;
  busy: boolean;
  onSubmit: (code: string) => void;
  onBack: () => void;
}) {
  return (
    <TextStep
      {...props}
      title={t('FLOW_CODE_TITLE')}
      body={t('FLOW_CODE_BODY', { email })}
      placeholder={t('FLOW_CODE_PLACEHOLDER')}
      action={t('FLOW_CODE_VERIFY')}
      keyboardType="number-pad"
      maxLength={8}
      valid={isValidCode}
    />
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  center: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  signInScroll: {
    flexGrow: 1,
    justifyContent: 'space-between',
    paddingTop: u(165),
    paddingBottom: u(82.5),
  },
  flowTitle: {
    fontSize: u(36),
    lineHeight: u(43.5),
    textAlign: 'center',
    marginTop: u(18),
  },
  logo: { width: u(160), height: u(160) },
  guestCreate: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(12),
  },
  guestCreateTitle: {
    fontSize: u(19),
    lineHeight: u(23),
    textAlign: 'center',
    width: u(174),
  },
  accountContainer: {
    flex: 1,
    justifyContent: 'space-between',
    paddingTop: u(110),
    paddingBottom: u(82.5),
  },
  brandMark: { width: u(55), height: u(55), alignSelf: 'center' },
  accountBody: {
    fontSize: u(20),
    lineHeight: u(24),
    marginTop: u(10),
    textAlign: 'center',
    maxWidth: u(267),
    alignSelf: 'center',
    color: colors.white,
  },
  accountActions: { gap: u(24) },
  accountButtons: { gap: u(24) },
  title: { marginTop: u(16), textAlign: 'center' },
  titleLeft: { marginTop: u(4), marginBottom: u(8) },
  copy: {
    marginTop: u(8),
    textAlign: 'center',
    opacity: 0.85,
    paddingHorizontal: u(16),
  },
  disclaimer: {
    lineHeight: u(18),
    textAlign: 'center',
    color: colors.snow,
  },
  disclaimerLink: {
    color: colors.link,
    textDecorationLine: 'underline',
  },
  error: { marginTop: u(12), color: colors.ruby },
  actions: { gap: u(23) },
  dividerRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(12),
    marginVertical: u(4),
  },
  dividerLine: { flex: 1, height: 1, backgroundColor: colors.white30 },
  dividerLabel: {
    textTransform: 'none',
    color: colors.pearl,
    fontFamily: 'Inter-Regular',
  },
  input: {
    height: u(34),
    borderRadius: u(6),
    paddingHorizontal: u(16),
    color: colors.snow,
    fontFamily: 'Inter-Medium',
    fontSize: u(15),
    backgroundColor: colors.black40,
    borderWidth: 1,
    borderColor: 'transparent',
  },
  inputFocused: { borderColor: colors.lavender },
  inputError: { borderColor: colors.ruby },
  inputGap: { marginBottom: u(16) },
  hint: { marginTop: u(8), marginBottom: u(16), textAlign: 'center' },
});
