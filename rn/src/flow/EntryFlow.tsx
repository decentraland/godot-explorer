import { useCallback, useEffect, useReducer, useRef, useState } from 'react';
import { BackHandler } from 'react-native';
import {
  acceptTerms,
  alreadySignedIn,
  authCode,
  cancelSignIn,
  nameTag,
  sendEmailCode,
  setAvatarName,
  signOut,
  startBrowserSignIn,
  startGuest,
  verifyEmailCode,
  welcome,
  whenSignedIn,
} from './auth';
import { core } from '../core';
import { UpdateDecision } from '../core/native';
import { UpdateAvailable } from './UpdateAvailable';
import { AvatarCustomize } from './AvatarCustomize';
import { takePreviewLaunch, whenInitialLinkRouted } from './launchLink';
import { askNotificationPermission } from '../notifications/push';
import { AuthOtpEntercode } from './AuthOtpEntercode';
import { initialFlow, reduceFlow } from './flowMachine';
import { AvatarCreate, PreviewArea, Comeback } from './previewScreens';
import {
  AccountHome,
  AuthOtpStart,
  AccountGuestCreate,
  AvatarNaming,
  AuthHome,
  DclSplash,
  AuthBrowserOpen,
  AuthVerify,
} from './screens';
import { t } from '../i18n';

/** Portrait-only entry flow: splash → account → (auth) → avatar → name → app. */
export function EntryFlow({ onDone }: { onDone: () => void }) {
  const [flow, dispatch] = useReducer(reduceFlow, initialFlow);
  const { step, provider } = flow;

  useEffect(() => {
    if (step === 'done') {
      onDone();
    }
  }, [step, onDone]);

  const [busy, setBusy] = useState(false);
  const [preset, setPreset] = useState(0);
  const [emailError, setEmailError] = useState<string | null>(null);
  const [resending, setResending] = useState(false);
  // Figma: "Opening browser..." until the browser hands the login back, then "Loading..." while the
  // profile is read.
  const [returned, setReturned] = useState(false);
  useEffect(() => setReturned(false), [step]);

  useEffect(() => {
    let cancelled = false;
    const succeeded = async () => {
      acceptTerms().catch(() => {});
      setReturned(true);
      try {
        const result = await welcome();
        if (!cancelled) {
          dispatch({ type: 'authSucceeded', ...result });
        }
      } catch (error) {
        failed(error);
      }
    };
    const failed = (error?: unknown) =>
      !cancelled &&
      dispatch({
        type: 'authFailed',
        message:
          error instanceof Error && error.message === 'sign_in_other_request'
            ? t('AUTH_SIGN_IN_OTHER_REQUEST')
            : t('FLOW_SIGN_IN_FAILED'),
      });
    if (
      step === 'auth_browser_open' &&
      (provider === 'google' || provider === 'apple' || provider === 'wallet')
    ) {
      try {
        startBrowserSignIn(provider === 'wallet' ? undefined : provider);
      } catch {
        // A recovered session can land while the user taps: the core refuses to begin a sign-in
        // when one is already done, and the flow simply moves on.
        if (!alreadySignedIn()) {
          failed();
          return () => {
            cancelled = true;
          };
        }
      }
      whenSignedIn().then(succeeded, failed);
    } else if (step === 'account_guest_create') {
      startGuest().then(
        () => !cancelled && dispatch({ type: 'guestReady' }),
        () => (alreadySignedIn() ? succeeded() : failed()),
      );
    }
    return () => {
      cancelled = true;
    };
  }, [step, provider]);

  const submitEmail = useCallback(async (email: string) => {
    setBusy(true);
    setEmailError(null);
    try {
      await sendEmailCode(email);
      dispatch({ type: 'emailSent', email });
    } catch {
      setEmailError(t('FLOW_EMAIL_SEND_FAILED'));
    } finally {
      setBusy(false);
    }
  }, []);

  const resend = useCallback(async () => {
    setResending(true);
    try {
      await submitEmail(flow.email);
    } finally {
      setResending(false);
    }
  }, [submitEmail, flow.email]);

  const submitCode = useCallback(
    async (code: string) => {
      setBusy(true);
      setEmailError(null);
      try {
        await verifyEmailCode(flow.email, code);
      } catch (e) {
        setEmailError(
          authCode(e) === 'invalid_code'
            ? t('FLOW_CODE_INVALID')
            : t('FLOW_SIGN_IN_FAILED'),
        );
        setBusy(false);
        return;
      }
      setBusy(false);
      // The code was right: "Confirming login..." until the profile is read.
      dispatch({ type: 'codeAccepted' });
      acceptTerms().catch(() => {});
      try {
        dispatch({ type: 'authSucceeded', ...(await welcome()) });
      } catch {
        dispatch({ type: 'authFailed', message: t('FLOW_SIGN_IN_FAILED') });
      }
    },
    [flow.email],
  );

  const back = useCallback(() => {
    // These return to the avatar, where the session must stay. The others leave a sign-in:
    // cancelled while it runs, signed out once it is done (a guest, or an account to name).
    if (step !== 'avatar_customize' && step !== 'avatar_naming') {
      if (alreadySignedIn()) {
        signOut();
      } else {
        cancelSignIn();
      }
    }
    setEmailError(null);
    // Backing out of the avatar ends the session: the next one starts on the first look again.
    if (step === 'avatar_create') {
      setPreset(0);
    }
    dispatch({ type: 'back' });
  }, [step]);

  useEffect(() => {
    const subscription = BackHandler.addEventListener(
      'hardwareBackPress',
      () => {
        if (step === 'dcl_splash' || step === 'account_home') {
          return false;
        }
        // A required update has nowhere to go back to.
        if (step === 'update_available') {
          return true;
        }
        // The customize step asks before discarding; its own listener runs next.
        if (step === 'avatar_customize') {
          return false;
        }
        back();
        return true;
      },
    );
    return () => subscription.remove();
  }, [step, back]);

  const gateChecked = useRef(false);
  const booted = useCallback(async () => {
    await whenInitialLinkRouted();
    // The version gate asks once per launch, before the account screens; LATER comes back here.
    if (!gateChecked.current) {
      gateChecked.current = true;
      const gate = await core()
        .versionGate()
        .catch(() => null);
      if (gate && gate.decision !== UpdateDecision.Proceed) {
        dispatch({
          type: 'updateGate',
          required: gate.decision === UpdateDecision.Required,
        });
        return;
      }
    }
    const preview = takePreviewLaunch();
    if (core().state().tag === 'SignedIn') {
      try {
        dispatch({ type: 'recovered', ...(await welcome()) });
        return;
      } catch {
        // The profile could not be read: fall through to the normal start.
      }
    }
    if (preview) {
      dispatch({ type: 'previewGuest' });
      return;
    }
    dispatch({ type: 'booted' });
  }, []);
  const confirmAvatar = useCallback(
    () => dispatch({ type: 'avatarConfirmed' }),
    [],
  );
  // The Account Home taps are the commit point: the terms are accepted and the OS asks for
  // notifications over the next screen, as the 1.x lobby did (Figma NOTIF_PROMPT).
  const commit = useCallback(() => {
    acceptTerms().catch(() => {});
    askNotificationPermission().catch(() => {});
  }, []);

  switch (step) {
    case 'dcl_splash':
      return <DclSplash onDone={booted} />;
    case 'update_available':
      return (
        <UpdateAvailable
          required={flow.updateRequired}
          onLater={() => {
            core().snoozeVersionGate();
            dispatch({ type: 'updateLater' });
          }}
        />
      );
    case 'account_home':
      return (
        <AccountHome
          onGuest={() => {
            commit();
            dispatch({ type: 'playAsGuest' });
          }}
          onSignIn={() => {
            commit();
            dispatch({ type: 'chooseSignIn' });
          }}
        />
      );
    case 'auth_home':
      return (
        <AuthHome
          error={flow.error}
          onProvider={next =>
            dispatch({ type: 'chooseProvider', provider: next })
          }
          onBack={back}
        />
      );
    case 'auth_otp_start':
    case 'auth_otp_entercode': {
      // The code is asked in a modal over the email screen, which stays where it is.
      const coding = step === 'auth_otp_entercode';
      return (
        <>
          <AuthOtpStart
            initial={flow.email}
            error={coding ? null : emailError}
            busy={busy && !coding}
            onSubmit={submitEmail}
            onBack={back}
          />
          <AuthOtpEntercode
            visible={coding}
            email={flow.email}
            error={coding ? emailError : null}
            busy={busy && !resending}
            onSubmit={submitCode}
            onResend={resend}
            onClose={back}
          />
        </>
      );
    }
    case 'auth_browser_open':
      return <AuthBrowserOpen loading={returned} onCancel={back} />;
    case 'auth_verify':
      return <AuthVerify />;
    case 'account_guest_create':
      return <AccountGuestCreate />;
    case 'comeback':
      return (
        <Comeback
          name={flow.name}
          onContinue={() => dispatch({ type: 'finish' })}
          onUseAnother={back}
        />
      );
    case 'avatar_create':
      return (
        <AvatarCreate
          selected={preset}
          onSelect={setPreset}
          onCustomize={() => dispatch({ type: 'customizeAvatar' })}
          onContinue={confirmAvatar}
          onBack={back}
        />
      );
    case 'avatar_customize':
      return <AvatarCustomize onDone={confirmAvatar} onDiscard={back} />;
    case 'avatar_naming':
      return (
        <AvatarNaming
          tag={nameTag()}
          preview={<PreviewArea stage="naming" />}
          onBack={back}
          onSubmit={name => {
            setAvatarName(name);
            // Only an account without a profile names an avatar here: it is a new player.
            try {
              core().ftueMarkNewPlayer();
            } catch {
              // Signed out meanwhile: there is no account to remember.
            }
            dispatch({ type: 'named', name });
          }}
        />
      );
    default:
      return null;
  }
}
