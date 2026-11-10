import { Image, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { PressableBase } from './internal/PressableBase';
import { colors, fonts, radii } from './tokens';
import { u } from './scale';

const walletConnectLogo = require('../assets/wallet-connect.png');

export type LoginProvider = 'google' | 'apple' | 'wallet' | 'email';

/** Figma's `Button/Entry` family — the Sign In screen's provider buttons. One real component with
 * four `Type`s (Google/Apple/Wallet/Mail), not four bespoke buttons: Google/Apple/Wallet share the
 * white/`DCL Shadow` styling ("light"), Mail is the one dark variant (Figma's own note: "Mail uses
 * variant=secondary"). Icon-to-label gap is per-brand in Figma (10px Wallet, 8px Mail, 4px
 * Google/Apple), not a single shared value. */
export function LoginButton({
  provider,
  label,
  onPress,
  disabled = false,
  fullWidth = false,
}: {
  provider: LoginProvider;
  label: string;
  onPress: () => void;
  disabled?: boolean;
  /** Figma's `Fill container` width mode (default is `Hug`). */
  fullWidth?: boolean;
}) {
  const spec = PROVIDER[provider];
  const contentColor = spec.dark && disabled ? colors.textDisabled : spec.color;
  return (
    <PressableBase
      onPress={onPress}
      disabled={disabled}
      accessibilityLabel={label}
      style={pressed => [
        styles.button,
        fullWidth && styles.fullWidth,
        {
          backgroundColor: disabled
            ? spec.disabledBg
            : pressed
            ? spec.pressedBg
            : spec.bg,
        },
      ]}
    >
      <View
        style={[
          styles.content,
          { gap: spec.gap },
          // "Light" providers (Google/Apple/Wallet) keep DCL Shadow text even when disabled —
          // Figma dims the whole content to 60% instead of swapping the text colour, unlike
          // Mail's explicit `theme.colors.text.disabled`.
          !spec.dark && disabled && styles.lightDisabledContent,
        ]}
      >
        {spec.icon === 'wallet' ? (
          <Image
            source={walletConnectLogo}
            style={styles.icon}
            resizeMode="contain"
          />
        ) : (
          <Icon
            name={spec.icon}
            size={spec.iconSize}
            color={spec.dark ? contentColor : undefined}
          />
        )}
        <AppText
          variant="button"
          style={[styles.label, { color: contentColor }]}
        >
          {label}
        </AppText>
      </View>
    </PressableBase>
  );
}

const PROVIDER: Record<
  LoginProvider,
  {
    icon: 'google' | 'apple' | 'mail' | 'wallet';
    iconSize: number;
    bg: string;
    pressedBg: string;
    disabledBg: string;
    color: string;
    gap: number;
    /** Mail is the one dark/"secondary" variant — its disabled state swaps the text colour
     * instead of dimming content opacity, like `Button`'s secondary tone. */
    dark?: boolean;
  }
> = {
  google: {
    icon: 'google',
    iconSize: 18,
    bg: colors.snow,
    pressedBg: colors.white70,
    disabledBg: colors.white10,
    color: colors.shadow,
    gap: u(2),
  },
  apple: {
    icon: 'apple',
    iconSize: 18,
    bg: colors.snow,
    pressedBg: colors.white70,
    disabledBg: colors.white10,
    color: colors.shadow,
    gap: u(2),
  },
  wallet: {
    icon: 'wallet',
    iconSize: 18,
    bg: colors.snow,
    pressedBg: colors.white70,
    disabledBg: colors.white10,
    color: colors.shadow,
    gap: u(5),
  },
  email: {
    icon: 'mail',
    iconSize: 19,
    bg: colors.black40,
    pressedBg: colors.black60,
    disabledBg: colors.black30,
    color: colors.snow,
    gap: u(4),
    dark: true,
  },
};

const styles = StyleSheet.create({
  button: {
    alignItems: 'center',
    justifyContent: 'center',
    minHeight: u(48),
    paddingHorizontal: u(12),
    borderRadius: radii.button,
  },
  fullWidth: { alignSelf: 'stretch' },
  content: { flexDirection: 'row', alignItems: 'center' },
  icon: { width: u(18), height: u(18) },
  label: { fontFamily: fonts.bold, textTransform: 'uppercase' },
  lightDisabledContent: { opacity: 0.6 },
});
