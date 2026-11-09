import { useCallback, useEffect, useReducer, useState } from 'react';
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
import { CustomizeStep } from './CustomizeStep';
import { takePreviewLaunch, whenInitialLinkRouted } from './launchLink';
import { askNotificationPermission } from '../notifications/push';
import { initialFlow, reduceFlow } from './flowMachine';
import {
  AvatarCreateScreen,
  PreviewArea,
  WelcomeBackScreen,
} from './previewScreens';
import {
  AccountScreen,
  CodeScreen,
  EmailScreen,
  GuestCreateScreen,
  NamingScreen,
  SignInScreen,
  SplashScreen,
  WaitScreen,
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

  useEffect(() => {
    let cancelled = false;
    const succeeded = async () => {
      acceptTerms().catch(() => {});
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
      step === 'auth_wait' &&
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
    } else if (step === 'guest_wait') {
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

  const submitCode = useCallback(
    async (code: string) => {
      setBusy(true);
      setEmailError(null);
      try {
        await verifyEmailCode(flow.email, code);
        acceptTerms().catch(() => {});
        dispatch({ type: 'authSucceeded', ...(await welcome()) });
      } catch (e) {
        setEmailError(
          authCode(e) === 'invalid_code'
            ? t('FLOW_CODE_INVALID')
            : t('FLOW_SIGN_IN_FAILED'),
        );
      } finally {
        setBusy(false);
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
        if (step === 'splash' || step === 'account') {
          return false;
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

  const booted = useCallback(async () => {
    await whenInitialLinkRouted();
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
    case 'splash':
      return <SplashScreen onDone={booted} />;
    case 'account':
      return (
        <AccountScreen
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
    case 'sign_in':
      return (
        <SignInScreen
          error={flow.error}
          onProvider={next =>
            dispatch({ type: 'chooseProvider', provider: next })
          }
          onBack={back}
        />
      );
    case 'email_entry':
      return (
        <EmailScreen
          initial={flow.email}
          error={emailError}
          busy={busy}
          onSubmit={submitEmail}
          onBack={back}
        />
      );
    case 'email_code':
      return (
        <CodeScreen
          email={flow.email}
          error={emailError}
          busy={busy}
          onSubmit={submitCode}
          onBack={back}
        />
      );
    case 'auth_wait':
      return (
        <WaitScreen
          title={t('FLOW_AUTH_WAIT_TITLE')}
          body={t('FLOW_AUTH_WAIT_BODY')}
          onCancel={back}
        />
      );
    case 'guest_wait':
      return <GuestCreateScreen />;
    case 'welcome_back':
      return (
        <WelcomeBackScreen
          name={flow.name}
          onContinue={() => dispatch({ type: 'finish' })}
        />
      );
    case 'avatar_create':
      return (
        <AvatarCreateScreen
          selected={preset}
          onSelect={setPreset}
          onCustomize={() => dispatch({ type: 'customizeAvatar' })}
          onContinue={confirmAvatar}
          onBack={back}
        />
      );
    case 'avatar_customize':
      return <CustomizeStep onDone={confirmAvatar} onDiscard={back} />;
    case 'avatar_naming':
      return (
        <NamingScreen
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
