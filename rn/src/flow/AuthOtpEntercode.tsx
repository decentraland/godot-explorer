import { useEffect, useState } from 'react';
import { Text } from 'react-native';
import { fonts, OtpDialog } from '@dcl/ui-native';
import { t } from '../i18n';

export const CODE_LENGTH = 6;
export const isValidCode = (code: string) =>
  new RegExp(`^\\d{${CODE_LENGTH}}$`).test(code.trim());

/** Figma AUTH_OTP_ENTERCODE / _VERIFY / _ERROR: the one-time code over the email screen, drawn by
 * the design system's `OtpDialog`; the sixth digit submits it. */
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

  // The address is bold inside the sentence, whatever order the translation puts it in.
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
    <OtpDialog
      visible={visible}
      state={failed ? 'error' : busy ? 'verifying' : 'default'}
      title={t('FLOW_CODE_TITLE')}
      body={
        <>
          {before}
          <Text style={{ fontFamily: fonts.bold }}>{email}</Text>
          {after}
        </>
      }
      code={code}
      length={CODE_LENGTH}
      onChangeCode={change}
      codeLabel={t('FLOW_CODE_TITLE')}
      errorMessage={error ?? ''}
      verifyingLabel={t('FLOW_CODE_VERIFYING')}
      resendPrompt={t('FLOW_CODE_RESEND_PROMPT')}
      resendLabel={
        resendOff
          ? t('FLOW_CODE_RESEND_WAIT', { time: countdown })
          : t('FLOW_CODE_RESEND')
      }
      resendDisabled={resendOff}
      onResend={onResend}
      onRequestClose={onClose}
    />
  );
}
