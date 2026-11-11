import { ReactNode, useEffect, useRef, useState } from 'react';
import {
  Image,
  Keyboard,
  Linking,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { core } from '../core';
import {
  AppText,
  colors,
  Button,
  Icon,
  LoginButton,
  Spinner,
  TextField,
  u,
} from '@dcl/ui-native';
import { FlowFrame } from './FlowFrame';
import { isValidName, NAME_MAX, Provider } from './flowMachine';
import { randomName } from './randomName';
import { I18nKey, t } from '../i18n';

/** Figma `Loader/Logo`: the logo animated, 154 px source drawn at the splash's 130. */
const loadingLogo = require('../assets/logo-loading.gif');
const avatarShadow = require('../assets/avatar-shadow.png');

export function DclSplash({ onDone }: { onDone: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onDone, 1200);
    return () => clearTimeout(timer);
  }, [onDone]);
  return (
    <FlowFrame flat edgeToEdge>
      <View style={styles.center}>
        <Image source={loadingLogo} style={styles.logo} resizeMode="contain" />
      </View>
    </FlowFrame>
  );
}

/** Inline "Terms of Service, Privacy Policy, and Content Policy" disclaimer with real links: underlined
 * on ACCOUNT_HOME, plain on AUTH_HOME, as the two frames draw it. */
function TermsDisclaimer({ underline = false }: { underline?: boolean }) {
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
              style={[styles.disclaimerLink, underline && styles.underlined]}
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

export function AccountHome({
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
          <View style={styles.brandMark}>
            <Icon name="nameLogo" size={50} />
          </View>
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
          <TermsDisclaimer underline />
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

export function AuthHome({
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
        <AppText
          variant="heading"
          style={[styles.flowTitle, styles.signInTitle]}
        >
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
export function AccountGuestCreate() {
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

/** Figma AUTH_BROWSER_OPEN 1564:9629: the browser is opening for a provider, CANCEL comes back.
 * `loading` is the 2582:15409 version, shown when the person returns from the browser. */
const LOADING_ESCAPE_MS = 8000;

export function AuthBrowserOpen({
  loading = false,
  onCancel,
}: {
  loading?: boolean;
  onCancel?: () => void;
}) {
  const title = t(loading ? 'FLOW_AUTH_LOADING_TITLE' : 'FLOW_AUTH_WAIT_TITLE');
  // Closing the browser without finishing sends no event: after a while Loading offers the way out.
  const [stuck, setStuck] = useState(false);
  useEffect(() => {
    setStuck(false);
    if (!loading) {
      return;
    }
    const timer = setTimeout(() => setStuck(true), LOADING_ESCAPE_MS);
    return () => clearTimeout(timer);
  }, [loading]);
  return (
    <FlowFrame edgeToEdge>
      <View style={styles.waitScroll}>
        <AppText
          variant="heading"
          style={[styles.flowTitle, styles.signInTitle]}
        >
          {title}
        </AppText>
        <View style={styles.waitSpinner}>
          <Spinner />
        </View>
      </View>
      {onCancel && (!loading || stuck) ? (
        <View style={styles.waitCancel}>
          <Button
            big
            tone="secondary"
            fullWidth
            label={t('FLOW_CANCEL')}
            onPress={onCancel}
          />
        </View>
      ) : null}
    </FlowFrame>
  );
}

/** Figma AUTH_VERIFY 1842:14702: "Confirming login..." between the accepted code and the avatar. */
export function AuthVerify() {
  return (
    <FlowFrame edgeToEdge>
      <View style={styles.guestCreate}>
        <Spinner size={75} accessibilityLabel={t('FLOW_AUTH_VERIFY_TITLE')} />
        <AppText variant="heading" style={styles.guestCreateTitle}>
          {t('FLOW_AUTH_VERIFY_TITLE')}
        </AppText>
      </View>
    </FlowFrame>
  );
}

/** Figma `AVATAR_NAMING` (1897:13029): the name field with its `#tag` and a dice over the avatar
 * stage, LET'S GO at the bottom. */
export function AvatarNaming({
  initial,
  tag,
  preview,
  onSubmit,
  onBack,
}: {
  /** Seeds the field; a random name when absent, as the lobby did. */
  initial?: string;
  /** The account's `#1234` suffix. */
  tag: string;
  preview: ReactNode;
  onSubmit: (name: string) => void;
  onBack: () => void;
}) {
  const [name, setName] = useState(() => initial ?? randomName());
  return (
    <View style={styles.fill}>
      {preview}
      <Image source={avatarShadow} style={styles.namingShadow} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('APP_BACK')}
        onPress={onBack}
        hitSlop={12}
        style={styles.namingBack}
      >
        <Icon name="back" size={30} />
      </Pressable>
      <View style={styles.naming}>
        <AppText variant="heading" style={styles.namingTitle}>
          {t('FLOW_NAME_TITLE')}
        </AppText>
        <View style={styles.namingRow}>
          <View style={styles.fill}>
            <TextField
              label=""
              value={name}
              onChangeText={setName}
              maxLength={NAME_MAX}
              showCounter
              counterBelow
              helper={t('FLOW_NAME_HINT')}
              error={nameError(name)}
              autoCapitalize="none"
              suffix={
                tag ? (
                  <AppText variant="body" style={styles.namingTag}>
                    {`#${tag}`}
                  </AppText>
                ) : undefined
              }
            />
          </View>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('FLOW_NAME_RANDOMIZE')}
            onPress={() => setName(randomName())}
            style={styles.namingDice}
          >
            <Icon name="dice" size={26} />
          </Pressable>
        </View>
      </View>
      <View style={styles.namingFooter}>
        <Button
          big
          fullWidth
          label={t('FLOW_LETS_GO')}
          disabled={!isValidName(name)}
          onPress={() => onSubmit(name)}
        />
      </View>
    </View>
  );
}

const nameError = (name: string): string | undefined => {
  if (name.length === 0 || isValidName(name)) {
    return undefined;
  }
  return /^\s|\s$/.test(name)
    ? t('INPUTS_NO_EDGE_SPACES')
    : t('FLOW_NAME_INVALID');
};

/** One text field and a button: the email address, then the code it was sent. */
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
export const isValidEmail = (email: string) => EMAIL_PATTERN.test(email.trim());

/** Figma AUTH_OTP_START 1842:13788 / 1842:14061 / 1842:14023: the email, then NEXT. With the keyboard
 * up, the field and the button rise to keep 33 px above it (the frame's annotation); the code is
 * asked in a modal over this screen. */
export function AuthOtpStart({
  initial,
  error,
  busy,
  onSubmit,
  onBack,
}: {
  initial: string;
  error: string | null;
  busy: boolean;
  onSubmit: (email: string) => void;
  onBack: () => void;
}) {
  const [email, setEmail] = useState(initial);
  const [blurred, setBlurred] = useState(false);
  const form = useRef<View>(null);
  const formBottom = useRef(0);
  const [lift, setLift] = useState(0);
  const measure = () =>
    form.current?.measureInWindow((_x, y, _w, h) => {
      formBottom.current = y + h + formLift.current;
    });
  // The form's own offset is undone before measuring, so the lift never feeds back into itself.
  const formLift = useRef(0);
  formLift.current = lift;
  useEffect(() => {
    const ios = Platform.OS === 'ios';
    const show = Keyboard.addListener(
      ios ? 'keyboardWillShow' : 'keyboardDidShow',
      event =>
        setLift(
          Math.max(
            0,
            formBottom.current + u(16.5) - event.endCoordinates.screenY,
          ),
        ),
    );
    const hide = Keyboard.addListener(
      ios ? 'keyboardWillHide' : 'keyboardDidHide',
      () => setLift(0),
    );
    return () => {
      show.remove();
      hide.remove();
    };
  }, []);
  const invalid = blurred && email.trim() !== '' && !isValidEmail(email);
  const filled = email.trim() !== '' && !error && !invalid;
  return (
    <FlowFrame onBack={onBack} edgeToEdge>
      <View style={styles.emailScroll}>
        <AppText
          variant="heading"
          style={[styles.flowTitle, styles.signInTitle]}
        >
          {t('FLOW_EMAIL_TITLE')}
        </AppText>
        <View
          ref={form}
          onLayout={measure}
          style={[
            styles.emailForm,
            // Figma `Input/Field` State=Filled keeps NEXT 8 px further down than Default and Error.
            filled && styles.emailFormFilled,
            { transform: [{ translateY: -lift }] },
          ]}
        >
          <TextField
            label={t('FLOW_EMAIL_LABEL')}
            value={email}
            onChangeText={setEmail}
            placeholder={t('FLOW_EMAIL_PLACEHOLDER')}
            keyboardType="email-address"
            autoCapitalize="none"
            error={error ?? (invalid ? t('FLOW_EMAIL_INVALID') : undefined)}
            onClear={() => setEmail('')}
            onFocusChange={focused => {
              if (!focused) {
                setBlurred(true);
              }
            }}
          />
          <Button
            big
            fullWidth
            label={t('FLOW_NEXT')}
            disabled={busy || email.trim() === ''}
            onPress={() => {
              // NEXT stays on while there is text: a bad format shows the error instead of sending.
              if (isValidEmail(email)) {
                onSubmit(email.trim());
              } else {
                setBlurred(true);
              }
            }}
          />
        </View>
      </View>
    </FlowFrame>
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1 },
  center: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  signInScroll: {
    flexGrow: 1,
    justifyContent: 'space-between',
    paddingTop: u(165),
    paddingBottom: u(74.5),
  },
  flowTitle: {
    fontSize: u(36),
    lineHeight: u(43.5),
    textAlign: 'center',
    marginTop: u(18),
  },
  signInTitle: { marginTop: 0 },
  emailScroll: { paddingTop: u(165) },
  /** Figma AUTH_BROWSER_OPEN: the title at y 330, the 150 px spinner from y 776, CANCEL at y 1198. */
  waitScroll: { paddingTop: u(165) },
  waitSpinner: { alignItems: 'center', marginTop: u(136) },
  waitCancel: { position: 'absolute', left: u(32), right: u(32), top: u(599) },
  /** Figma: the label starts at y 654, 150 px under the title; NEXT 40 px under the field. */
  /** The form spans 624 px (48 px margins), 16 px wider each side than the 64 px title column. */
  emailForm: { marginTop: u(75), gap: u(28), marginHorizontal: -u(8) },
  emailFormFilled: { gap: u(32) },
  /** Figma `Loader/Logo`: the GIF drawn at 140 px, so its 143 px circle (of 154) measures the 130 of the static logo. */
  logo: { width: u(70), height: u(70) },
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
  /** Figma `Logo` 100 px, the DS vector. */
  brandMark: { alignSelf: 'center' },
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
  disclaimer: {
    lineHeight: u(18),
    textAlign: 'center',
    color: colors.snow,
  },
  disclaimerLink: { color: colors.link },
  underlined: { textDecorationLine: 'underline' },
  error: { marginTop: u(12), color: colors.ruby },
  actions: { gap: u(23) },
  dividerRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(16),
    height: u(15.5),
  },
  dividerLine: { flex: 1, height: u(1), backgroundColor: colors.pearl },
  dividerLabel: {
    fontSize: u(12),
    lineHeight: u(14.5),
    textTransform: 'none',
    color: colors.pearl,
    fontFamily: 'Inter-Regular',
  },
  namingBack: { position: 'absolute', top: u(43), left: u(24) },
  /** Figma's blurred ellipse under the feet (318×45 plus its blur), centred at (337.5, 1288). */
  namingShadow: {
    position: 'absolute',
    left: u(70.75),
    top: u(614.25),
    width: u(196.2),
    height: u(59.65),
  },
  /** Figma AVATAR_NAMING `Name` (x 69, w 582): the title glyphs span y 208 to 241, the field starts at 258. */
  naming: {
    position: 'absolute',
    top: u(103),
    left: u(34.5),
    right: u(34.5),
    gap: u(8),
  },
  /** The 36 px title in the font's own line (Android clips descenders to a shorter one); the extra
   * height is taken back so the frame's positions hold. */
  namingTitle: {
    fontSize: u(18),
    lineHeight: u(22),
    marginTop: -u(2.5),
    marginBottom: -u(1.5),
  },
  namingRow: { flexDirection: 'row', alignItems: 'flex-start', gap: u(6.5) },
  namingTag: { fontSize: u(12), lineHeight: u(12), color: colors.snow },
  namingDice: {
    width: u(34.35),
    height: u(34),
    borderRadius: u(5.5),
    backgroundColor: colors.ruby,
    alignItems: 'center',
    justifyContent: 'center',
  },
  namingFooter: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    paddingHorizontal: u(24),
    paddingBottom: u(43),
  },
});
