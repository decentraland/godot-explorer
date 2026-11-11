import { useEffect, useRef, useState } from 'react';
import {
  Dimensions,
  Keyboard,
  Modal,
  NativeSyntheticEvent,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  TextLayoutEventData,
  View,
} from 'react-native';
import {
  AppText,
  colors,
  fonts,
  GLOW_BOTTOM,
  GLOW_TOP,
  Icon,
  RadialFill,
  Spinner,
  u,
} from '@dcl/ui-native';
import { t } from '../i18n';

export const CODE_LENGTH = 6;
export const isValidCode = (code: string) =>
  new RegExp(`^\\d{${CODE_LENGTH}}$`).test(code.trim());

/** Figma AUTH_OTP_ENTERCODE / _VERIFY / _ERROR: the one-time code over the email screen. The code
 * is typed into a hidden field and drawn in six slots; the sixth digit submits it. */
export function AuthOtpEntercode({
  visible,
  email,
  error,
  busy,
  onSubmit,
  onResend,
  onClose,
  initialCode = '',
  resendAt = 0,
}: {
  visible: boolean;
  email: string;
  error: string | null;
  busy: boolean;
  onSubmit: (code: string) => void;
  onResend: () => void;
  onClose: () => void;
  /** The digits the slots start with, for the gallery. */
  initialCode?: string;
  /** When the link text turns on again (epoch ms): Figma disables it for 2 minutes after each send. */
  resendAt?: number;
}) {
  const input = useRef<TextInput>(null);
  const [code, setCode] = useState(initialCode);
  // The red state belongs to the code that failed: typing again clears it.
  const [edited, setEdited] = useState(false);
  useEffect(() => {
    if (visible) {
      setCode(initialCode);
    }
  }, [visible, initialCode]);
  useEffect(() => setEdited(false), [error]);
  const failed = !!error && !edited && !busy;
  // Figma: centred; with the numeric keyboard up the card rests 50 px above it (top at y 140).
  const [keyboardTop, setKeyboardTop] = useState<number | null>(null);
  useEffect(() => {
    const show = Keyboard.addListener('keyboardDidShow', event =>
      setKeyboardTop(event.endCoordinates.screenY),
    );
    const hide = Keyboard.addListener('keyboardDidHide', () =>
      setKeyboardTop(null),
    );
    return () => {
      show.remove();
      hide.remove();
    };
  }, []);
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (resendAt <= Date.now()) {
      return;
    }
    setNow(Date.now());
    const timer = setInterval(() => {
      setNow(Date.now());
      if (Date.now() >= resendAt) {
        clearInterval(timer);
      }
    }, 1000);
    return () => clearInterval(timer);
  }, [resendAt]);
  const resendOff = now < resendAt;
  const left = Math.max(0, Math.ceil((resendAt - now) / 1000));
  const countdown = `${Math.floor(left / 60)}:${String(left % 60).padStart(
    2,
    '0',
  )}`;
  // The address is bold inside the sentence, whatever order the translation puts it in. Android
  // may break a line between an address and the full stop after it, which the frame never does:
  // then the address starts the next line, as the frame draws it.
  const [pushed, setPushed] = useState(false);
  useEffect(() => setPushed(false), [email]);
  const checkStop = (event: NativeSyntheticEvent<TextLayoutEventData>) => {
    if (event.nativeEvent.lines.some(line => /^\s*\./.test(line.text))) {
      setPushed(true);
    }
  };
  const [before, after] = t('FLOW_CODE_BODY', { email: '\u0000' }).split(
    '\u0000',
  );

  const change = (text: string) => {
    const digits = text.replace(/\D/g, '').slice(0, CODE_LENGTH);
    setCode(digits);
    setEdited(true);
    if (digits.length === CODE_LENGTH) {
      onSubmit(digits);
    }
  };

  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      statusBarTranslucent
      onRequestClose={onClose}
    >
      <View
        style={[
          styles.scrim,
          keyboardTop !== null && {
            ...styles.scrimLifted,
            paddingBottom:
              Dimensions.get('window').height - keyboardTop + u(25),
          },
        ]}
      >
        <View style={styles.card} accessibilityViewIsModal>
          <View style={styles.clip} pointerEvents="none">
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
              cy={1.04}
              rxDp={u(171.5)}
              ryDp={u(70)}
            />
          </View>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_DISMISS_DIALOG')}
            hitSlop={12}
            onPress={onClose}
            style={styles.close}
          >
            <Icon name="close" size={24.5} />
          </Pressable>
          <View style={styles.icon}>
            <Icon name="mail" size={39} />
          </View>
          <View style={styles.text}>
            <AppText style={styles.title}>{t('FLOW_CODE_TITLE')}</AppText>
            <Text
              style={styles.body}
              textBreakStrategy="simple"
              onTextLayout={checkStop}
            >
              {pushed ? `${before.trimEnd()}\n` : before}
              <Text style={styles.email}>{email}</Text>
              {after}
            </Text>
          </View>
          <Pressable
            accessibilityRole="none"
            onPress={() => input.current?.focus()}
            style={styles.slots}
          >
            {Array.from({ length: CODE_LENGTH }, (_, i) => {
              // Figma `Input/CodeSlots` State=Typing: the slot the next digit lands in.
              const typing = !busy && !failed && i === code.length;
              return (
                <View
                  key={i}
                  style={[
                    styles.slot,
                    code[i] ? styles.slotFilled : null,
                    typing && styles.slotTyping,
                    failed && styles.slotFailed,
                  ]}
                >
                  {typing ? <View style={styles.caret} /> : null}
                  <Text style={[styles.digit, failed && styles.digitFailed]}>
                    {code[i] ?? ''}
                  </Text>
                </View>
              );
            })}
            <TextInput
              ref={input}
              value={code}
              onChangeText={change}
              autoFocus
              editable={!busy}
              keyboardType="number-pad"
              textContentType="oneTimeCode"
              autoComplete="one-time-code"
              maxLength={CODE_LENGTH}
              caretHidden
              style={styles.hiddenInput}
            />
          </Pressable>
          <View style={styles.status}>
            {failed ? <AppText style={styles.error}>{error}</AppText> : null}
            {busy ? (
              <View style={styles.verifying}>
                <Spinner size={24} />
                <AppText style={styles.verifyingText}>
                  {t('FLOW_CODE_VERIFYING')}
                </AppText>
              </View>
            ) : null}
          </View>
          <AppText style={styles.resend}>
            {t('FLOW_CODE_RESEND_PROMPT')}{' '}
            <Text
              accessibilityRole="link"
              accessibilityState={{ disabled: resendOff }}
              onPress={resendOff ? undefined : onResend}
              style={[styles.resendLink, resendOff && styles.resendOff]}
            >
              {resendOff
                ? t('FLOW_CODE_RESEND_WAIT', { time: countdown })
                : t('FLOW_CODE_RESEND')}
            </Text>
          </AppText>
        </View>
      </View>
    </Modal>
  );
}

/** Figma `MOdal-OTP` 620×810, centred on the 720×1600 frame (y 395), every size in px / 2. */
const styles = StyleSheet.create({
  scrimLifted: { justifyContent: 'flex-end' },
  scrim: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.black40,
  },
  card: {
    width: u(310),
    paddingTop: u(48),
    paddingBottom: u(14),
    paddingHorizontal: u(16),
    borderRadius: u(8),
    backgroundColor: colors.modal,
    alignItems: 'center',
  },
  clip: {
    ...StyleSheet.absoluteFill,
    borderRadius: u(8),
    overflow: 'hidden',
  },
  /** Figma `Close`: the glyph is centred 47 px from the card's top and right edges. */
  close: { position: 'absolute', top: u(11.25), right: u(11.25) },
  icon: {
    width: u(78),
    height: u(78),
    borderRadius: u(39),
    backgroundColor: colors.black40,
    alignItems: 'center',
    justifyContent: 'center',
  },
  text: { alignSelf: 'stretch', marginTop: u(18), gap: u(11) },
  title: {
    fontSize: u(22),
    lineHeight: u(26.5),
    fontFamily: fonts.bold,
    textAlign: 'center',
    color: colors.snow,
  },
  // 8 px wider than the frame's 556 px box: Android measures the first sentence a hair longer.
  body: {
    marginHorizontal: -u(4),
    fontFamily: fonts.regular,
    fontSize: u(13),
    lineHeight: u(17),
    textAlign: 'center',
    color: colors.snow,
  },
  email: { fontFamily: fonts.bold },
  slots: {
    flexDirection: 'row',
    gap: u(5),
    marginTop: u(24),
    alignSelf: 'stretch',
    justifyContent: 'center',
  },
  slot: {
    width: u(42),
    height: u(65),
    borderRadius: u(8),
    // Figma strokes the slots at 1 px; the Typing one at 2.
    borderWidth: 0.5,
    borderColor: colors.shadow,
    backgroundColor: colors.black40,
    alignItems: 'center',
    justifyContent: 'center',
  },
  slotFilled: { borderColor: colors.pearl },
  slotTyping: {
    borderWidth: 1,
    borderColor: colors.lavender,
    backgroundColor: colors.black70,
  },
  caret: {
    position: 'absolute',
    width: 1,
    height: u(25),
    backgroundColor: colors.snow,
  },
  slotFailed: { borderColor: colors.danger },
  digit: {
    fontSize: u(24),
    lineHeight: u(29),
    fontFamily: fonts.semibold,
    color: colors.snow,
  },
  digitFailed: { color: colors.danger },
  hiddenInput: { ...StyleSheet.absoluteFill, opacity: 0 },
  status: { alignSelf: 'stretch', height: u(43) },
  error: {
    position: 'absolute',
    top: u(4),
    left: 0,
    right: 0,
    fontSize: u(10),
    lineHeight: u(12),
    textAlign: 'center',
    color: colors.ruby,
  },
  verifying: {
    position: 'absolute',
    top: u(19),
    left: 0,
    right: 0,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(3.5),
  },
  verifyingText: {
    fontSize: u(12),
    lineHeight: u(14.5),
    fontFamily: fonts.semibold,
  },
  resend: {
    marginTop: u(12),
    fontSize: u(12),
    lineHeight: u(14.5),
    textAlign: 'center',
    color: colors.snow,
  },
  resendOff: { color: colors.textDisabled },
  resendLink: {
    fontFamily: fonts.medium,
    color: colors.link,
    textDecorationLine: 'underline',
  },
});
