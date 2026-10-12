import { useCallback, useEffect, useReducer } from 'react';
import { BackHandler, StyleSheet, View } from 'react-native';
import { BackpackScreen } from '../backpack/BackpackScreen';
import { PrimaryButton } from '../ds';
import { send } from '../engine';
import { signIn, startGuestSession } from './auth';
import { FtueScreen } from './FtueScreen';
import { initialFlow, reduceFlow } from './flowMachine';
import {
  AvatarCreateScreen,
  PreviewArea,
  WelcomeBackScreen,
} from './previewScreens';
import {
  AccountScreen,
  NamingScreen,
  SignInScreen,
  SplashScreen,
  TermsScreen,
  WaitScreen,
} from './screens';
import { u } from '../ds/scale';

/** Portrait-only entry flow: splash → terms → account → (auth) → avatar → name → tour → app. */
export function EntryFlow({ onDone }: { onDone: () => void }) {
  const [flow, dispatch] = useReducer(reduceFlow, initialFlow);
  const { step, provider } = flow;

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

  useEffect(() => {
    let cancelled = false;
    if (step === 'auth_wait' && provider) {
      signIn(provider)
        .then(
          result =>
            !cancelled && dispatch({ type: 'authSucceeded', ...result }),
        )
        .catch(
          () =>
            !cancelled &&
            dispatch({
              type: 'authFailed',
              message: 'Could not sign in. Try again.',
            }),
        );
    } else if (step === 'guest_wait') {
      startGuestSession().then(
        () => !cancelled && dispatch({ type: 'guestReady' }),
      );
    }
    return () => {
      cancelled = true;
    };
  }, [step, provider]);

  const booted = useCallback(() => dispatch({ type: 'booted' }), []);
  const back = useCallback(() => dispatch({ type: 'back' }), []);
  const confirmAvatar = useCallback(
    () => dispatch({ type: 'avatarConfirmed' }),
    [],
  );

  switch (step) {
    case 'splash':
      return <SplashScreen onDone={booted} />;
    case 'terms':
      return <TermsScreen onAccept={() => dispatch({ type: 'acceptTerms' })} />;
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
    case 'auth_wait':
      return (
        <WaitScreen
          title="Signing you in"
          body="Hang tight while we finish up."
          onCancel={back}
        />
      );
    case 'guest_wait':
      return (
        <WaitScreen
          title="Getting you ready"
          body="Setting up your guest session."
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
          allowLandscape={false}
          backInPortrait
          onBack={back}
          footer={
            <View style={styles.footer}>
              <PrimaryButton label="Continue" onPress={confirmAvatar} />
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
            send({ type: 'set_name', name });
            dispatch({ type: 'named', name });
          }}
        />
      );
    case 'ftue':
      return <FtueScreen onDone={() => dispatch({ type: 'finish' })} />;
    default:
      return null;
  }
}

function PreviewFill() {
  return (
    <View style={styles.previewFill}>
      <PreviewArea />
    </View>
  );
}

const styles = StyleSheet.create({
  footer: { padding: u(24), paddingBottom: u(32), backgroundColor: '#161518' },
  previewFill: { flex: 1, marginVertical: u(8) },
});
