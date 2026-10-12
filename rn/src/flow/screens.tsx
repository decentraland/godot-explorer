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

const logo = require('../assets/logo.png');

const TERMS_URL = 'https://decentraland.org/terms';

export function SplashScreen({ onDone }: { onDone: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onDone, 1200);
    return () => clearTimeout(timer);
  }, [onDone]);
  return (
    <FlowFrame>
      <View style={styles.center}>
        <Image source={logo} style={styles.logo} resizeMode="contain" />
        <ActivityIndicator color={colors.snow} style={styles.spinner} />
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
          Before you start
        </AppText>
        <AppText variant="body" style={styles.copy}>
          By continuing you accept the Terms of Use and the Privacy Policy.
        </AppText>
        <Pressable
          accessibilityRole="link"
          onPress={() => Linking.openURL(TERMS_URL)}
          hitSlop={8}
        >
          <AppText variant="button" style={styles.link}>
            Read the terms
          </AppText>
        </Pressable>
      </View>
      <PrimaryButton label="Accept and continue" onPress={onAccept} />
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
      <View style={styles.center}>
        <Image source={logo} style={styles.logoSmall} resizeMode="contain" />
        <AppText variant="heading" style={styles.title}>
          Welcome to Decentraland
        </AppText>
        <AppText variant="body" style={styles.copy}>
          Jump in right away or sign in to keep your avatar.
        </AppText>
      </View>
      <View style={styles.actions}>
        <PrimaryButton label="Sign in or create account" onPress={onSignIn} />
        <PrimaryButton
          label="Play as guest"
          tone="secondary"
          onPress={onGuest}
        />
      </View>
    </FlowFrame>
  );
}

const PROVIDERS: { key: Provider; label: string }[] = [
  { key: 'google', label: 'Continue with Google' },
  { key: 'apple', label: 'Continue with Apple' },
  { key: 'email', label: 'Continue with email' },
  { key: 'wallet', label: 'Connect a wallet' },
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
          Sign in
        </AppText>
        <AppText variant="body" style={styles.copy}>
          Choose how you want to continue.
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
            label={provider.label}
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
        <PrimaryButton label="Cancel" tone="ghost" onPress={onCancel} />
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
          Choose your name
        </AppText>
        {preview}
        <TextInput
          value={name}
          onChangeText={setName}
          placeholder="Your name"
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
          3 to 15 letters or numbers
        </AppText>
        <PrimaryButton
          label="Let's go"
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
  logo: { width: 160, height: 160 },
  logoSmall: { width: 96, height: 96, marginBottom: 12 },
  spinner: { marginTop: 24 },
  title: { marginTop: 16, textAlign: 'center' },
  titleLeft: { marginTop: 4, marginBottom: 8 },
  copy: {
    marginTop: 8,
    textAlign: 'center',
    opacity: 0.85,
    paddingHorizontal: 16,
  },
  link: { marginTop: 14, color: colors.lavender },
  error: { marginTop: 12, color: colors.ruby },
  actions: { gap: 12 },
  input: {
    height: 52,
    borderRadius: radii.pill,
    paddingHorizontal: 20,
    color: colors.snow,
    fontFamily: 'Inter-Medium',
    fontSize: 16,
    backgroundColor: colors.white10,
    borderWidth: 1,
    borderColor: colors.white30,
  },
  hint: { marginTop: 8, marginBottom: 16, textAlign: 'center' },
});
