import { useCallback, useEffect, useReducer, useState } from 'react';
import { BackHandler, StyleSheet, View } from 'react-native';
import { BackpackScreen } from '../backpack/BackpackScreen';
import { PrimaryButton } from '../ds';
import type { Place } from '../discover/api';
import { useApi } from '../discover/ApiContext';
import { ftuePlaces } from '../discover/ftue';
import {
  acceptTerms,
  alreadySignedIn,
  authCode,
  cancelSignIn,
  sendEmailCode,
  setAvatarName,
  startBrowserSignIn,
  startGuest,
  termsAccepted,
  verifyEmailCode,
  welcome,
  whenSignedIn,
} from './auth';
import { core } from '../core';
import { takePreviewLaunch, whenInitialLinkRouted } from './launchLink';
import { initialFlow, reduceFlow } from './flowMachine';
import { FtueStep } from './FtueStep';
import {
  AvatarCreateScreen,
  PreviewArea,
  WelcomeBackScreen,
} from './previewScreens';
import {
  AccountScreen,
  CodeScreen,
  EmailScreen,
  NamingScreen,
  SignInScreen,
  SplashScreen,
  TermsScreen,
  WaitScreen,
} from './screens';
import { t } from '../i18n';
import { u } from '../ds/scale';

/** Portrait-only entry flow: splash → terms → account → (auth) → avatar → name → welcome → app. */
export function EntryFlow({
  onDone,
  onJumpIn,
}: {
  onDone: () => void;
  onJumpIn?: (place: Place) => Promise<boolean>;
}) {
  const [flow, dispatch] = useReducer(reduceFlow, initialFlow);
  const { step, provider } = flow;
  const api = useApi();

  // Ahead of the FTUE gate, so deciding the single-place bypass never waits on the network.
  useEffect(() => {
    if (step === 'avatar_create') {
      ftuePlaces(api);
    }
  }, [step, api]);

  useEffect(() => {
    const subscription = BackHandler.addEventListener(
      'hardwareBackPress',
      () => {
        if (step === 'splash' || step === 'terms') {
          return false;
        }
        dispatch({ type: 'back' });
        return true;
      },
    );
    return () => subscription.remove();
  }, [step]);

  useEffect(() => {
    if (step === 'done') {
      onDone();
    }
  }, [step, onDone]);

  // Only an account without a profile reaches the welcome after its avatar: it was made here.
  useEffect(() => {
    if (step === 'ftue') {
      try {
        core().ftueMarkNewPlayer();
      } catch {
        // Signed out meanwhile: there is no account to remember.
      }
    }
  }, [step]);

  const [busy, setBusy] = useState(false);
  const [emailError, setEmailError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const succeeded = async () =>
      !cancelled && dispatch({ type: 'authSucceeded', ...(await welcome()) });
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
    cancelSignIn();
    setEmailError(null);
    dispatch({ type: 'back' });
  }, []);

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
    dispatch({ type: 'booted', next: termsAccepted() ? 'account' : 'terms' });
  }, []);
  const confirmAvatar = useCallback(
    () => dispatch({ type: 'avatarConfirmed' }),
    [],
  );

  switch (step) {
    case 'splash':
      return <SplashScreen onDone={booted} />;
    case 'terms':
      return (
        <TermsScreen
          onAccept={() => {
            acceptTerms().catch(() => {});
            dispatch({ type: 'acceptTerms' });
          }}
        />
      );
    case 'account':
      return (
        <AccountScreen
          onGuest={() => dispatch({ type: 'playAsGuest' })}
          onSignIn={() => dispatch({ type: 'chooseSignIn' })}
          onBack={back}
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
      return (
        <WaitScreen
          title={t('FLOW_GUEST_WAIT_TITLE')}
          body={t('FLOW_GUEST_WAIT_BODY')}
          onCancel={back}
        />
      );
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
          onCustomize={() => dispatch({ type: 'customizeAvatar' })}
          onContinue={confirmAvatar}
          onBack={back}
        />
      );
    case 'avatar_customize':
      return (
        <BackpackScreen
          onBack={back}
          footer={
            <View style={styles.footer}>
              <PrimaryButton
                label={t('FLOW_CONTINUE')}
                onPress={confirmAvatar}
              />
            </View>
          }
        />
      );
    case 'avatar_naming':
      return (
        <NamingScreen
          initial={flow.name}
          preview={<PreviewFill />}
          onBack={back}
          onSubmit={name => {
            setAvatarName(name);
            dispatch({ type: 'named', name });
          }}
        />
      );
    case 'ftue':
      return (
        <FtueStep
          username={flow.name || t('FLOW_GUEST_NAME')}
          onJumpIn={onJumpIn}
          onFinish={() => dispatch({ type: 'finish' })}
        />
      );
    default:
      return null;
  }
}

function PreviewFill() {
  return (
    <View style={styles.previewFill}>
      <PreviewArea inset={false} />
    </View>
  );
}

const styles = StyleSheet.create({
  footer: { padding: u(24), paddingBottom: u(32), backgroundColor: '#161518' },
  previewFill: { flex: 1, marginVertical: u(8) },
});
