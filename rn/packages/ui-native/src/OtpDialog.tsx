import { ReactNode } from 'react';
import { Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { AppText } from './AppText';
import { CodeSlots } from './CodeSlots';
import { GLOW_BOTTOM, GLOW_TOP, useKeyboardLift } from './Dialog';
import { RadialFill } from './Gradient';
import { Icon } from './Icon';
import { Spinner } from './Spinner';
import { colors, fonts } from './tokens';
import { u } from './scale';
import { t } from './i18n';

export type OtpState = 'default' | 'verifying' | 'error';

/** Figma `MOdal-OTP` 891:15421: the e-mail verification code modal over `Modal/Scrim`, a 310 dp
 * `surface/modal` card with the Dialog glows, the mail badge, the `OTP-Input` slots and the resend
 * line, portrait only (the flow locks the orientation). `body` may nest a semibold `AppText` for the address. `verifying` shows the spinner row
 * between the slots and the resend line; `error` paints the slots and shows `errorMessage`. */
export function OtpDialog({
  visible,
  state,
  title,
  body,
  code,
  length = 6,
  onChangeCode,
  codeLabel,
  errorMessage,
  verifyingLabel,
  resendPrompt,
  resendLabel,
  onResend,
  resendDisabled = false,
  onRequestClose,
}: {
  visible: boolean;
  state: OtpState;
  title: string;
  body: ReactNode;
  code: string;
  length?: number;
  onChangeCode: (code: string) => void;
  codeLabel: string;
  errorMessage: string;
  verifyingLabel: string;
  /** "Didn’t get an email?" — the plain part before the link. */
  resendPrompt: string;
  /** "Resend Code", or the countdown ("00:30") while `resendDisabled`: `Message-CodeResent` Disable
   * keeps the link style. */
  resendLabel: string;
  onResend: () => void;
  resendDisabled?: boolean;
  onRequestClose: () => void;
}) {
  const lift = useKeyboardLift(visible);
  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      statusBarTranslucent
      onRequestClose={onRequestClose}
    >
      <View style={[styles.scrim, { paddingBottom: lift }]}>
        <Pressable
          accessibilityLabel={t('APP_DISMISS_DIALOG')}
          style={StyleSheet.absoluteFill}
          onPress={onRequestClose}
        />
        <View style={styles.card} accessibilityViewIsModal>
          <View style={styles.clip}>
            <RadialFill
              stops={GLOW_TOP}
              cx={1}
              cy={0}
              rxDp={u(105)}
              ryDp={u(95)}
            />
            <RadialFill
              stops={GLOW_BOTTOM}
              cx={0.5}
              cy={1.033}
              rxDp={u(171.5)}
              ryDp={u(70)}
            />
          </View>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_CLOSE')}
            hitSlop={u(12)}
            onPress={onRequestClose}
            style={styles.close}
          >
            <Icon name="close" size={26.75} />
          </Pressable>
          <View style={styles.content}>
            <View style={styles.header}>
              <View style={styles.badge}>
                <Icon name="email" size={39} />
              </View>
              <View style={styles.text}>
                <AppText style={styles.title}>{title}</AppText>
                <AppText style={styles.body} textBreakStrategy="simple">
                  {body}
                </AppText>
              </View>
            </View>
            <View style={styles.code}>
              <CodeSlots
                value={code}
                length={length}
                onChangeText={onChangeCode}
                error={state === 'error'}
                errorMessage={errorMessage}
                autoFocus
                accessibilityLabel={codeLabel}
              />
              <View style={styles.status}>
                {state === 'verifying' && (
                  <View style={styles.verifying}>
                    <Spinner size={24} />
                    <AppText style={styles.verifyingLabel}>
                      {verifyingLabel}
                    </AppText>
                  </View>
                )}
              </View>
              <View style={styles.resendRow}>
                <AppText style={styles.resend}>{resendPrompt} </AppText>
                <Text
                  accessibilityRole="link"
                  disabled={resendDisabled}
                  onPress={onResend}
                  style={[styles.resend, styles.link]}
                >
                  {resendLabel}
                </Text>
              </View>
            </View>
          </View>
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  scrim: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.black60,
  },
  card: {
    width: u(310),
    padding: u(16),
    // Figma: the resend line ends 14 dp above the card's bottom edge.
    paddingBottom: u(14),
    gap: u(16),
    borderRadius: u(8),
    backgroundColor: colors.modal,
  },
  clip: {
    ...StyleSheet.absoluteFill,
    borderRadius: u(8),
    overflow: 'hidden',
  },
  /** The node is the bare 15.86 dp glyph; the `close` icon box adds 11 of 54 units around it. */
  close: { alignSelf: 'flex-end', margin: -u(5.45), opacity: 0.9 },
  content: { gap: u(24), alignItems: 'center' },
  header: { alignSelf: 'stretch', alignItems: 'center', gap: u(18) },
  badge: {
    width: u(78),
    height: u(78),
    borderRadius: u(39),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.black40,
  },
  text: { alignSelf: 'stretch', gap: u(11) },
  title: {
    fontFamily: fonts.bold,
    fontSize: u(22),
    // The frame's 53 px title box; without a line height Android adds 2.5 dp.
    lineHeight: u(26.5),
    color: colors.snow,
    textAlign: 'center',
  },
  // 8 px wider than the frame's 556 px box: Android measures the first sentence a hair longer.
  body: {
    marginHorizontal: -u(4),
    fontSize: u(13),
    lineHeight: u(16.9),
    color: colors.snow,
    textAlign: 'center',
  },
  code: { alignItems: 'center' },
  /** The 39 dp between the slots and the resend line; Verifying puts its row 3 dp into it. */
  status: { height: u(39), paddingTop: u(3) },
  verifying: { flexDirection: 'row', alignItems: 'center', gap: u(3.5) },
  verifyingLabel: {
    fontFamily: fonts.semibold,
    fontSize: u(12),
    color: colors.snow,
  },
  resendRow: { flexDirection: 'row', alignItems: 'center', gap: u(5) },
  resend: { fontSize: u(12), lineHeight: u(14.5), color: colors.snow },
  link: {
    fontFamily: fonts.medium,
    color: colors.link,
    textDecorationLine: 'underline',
  },
});
