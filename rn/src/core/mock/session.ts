import { CoreError, SignInMethod, SignInStep, State } from '../native';
import type { AppListener } from '../native';
import type { SessionApi } from '../api/session';

const STEP_MS = 400;
const sleep = (ms: number) => new Promise<void>(resolve => setTimeout(resolve, ms));

/** A session that signs in after a short wait, for builds without the engine library. */
export function createMockSession(): SessionApi {
  let state: State = new State.SignedOut();
  let nextId = 0n;
  let wallet: string | undefined;
  const listeners = new Map<bigint, AppListener>();
  const emit = (next: State) => {
    state = next;
    listeners.forEach(l => l.stateChanged(next));
  };
  const signedIn = (address: string) => {
    wallet = address;
    emit(new State.SignedIn());
  };
  const needSignedOut = () => {
    if (state.tag !== 'SignedOut' && state.tag !== 'SigningIn') {
      throw new CoreError.InvalidCall({ detail: `not signed out: ${state.tag}` });
    }
  };

  return {
    state: () => state,
    address: () => wallet,
    installs: () => 1,
    addListener: listener => {
      listeners.set(nextId, listener);
      return nextId++;
    },
    removeListener: id => {
      listeners.delete(id);
    },
    signInGuest: async () => {
      needSignedOut();
      emit(new State.SigningIn({ method: SignInMethod.Guest, step: SignInStep.Starting }));
      await sleep(STEP_MS);
      signedIn('0xguest000000000000000000000000000000000001');
    },
    signInEmailStart: async () => {
      needSignedOut();
      emit(new State.SigningIn({ method: SignInMethod.Email, step: SignInStep.Starting }));
      await sleep(STEP_MS);
      emit(new State.SigningIn({ method: SignInMethod.Email, step: SignInStep.AwaitingCode }));
    },
    signInEmailVerify: async (_email, code) => {
      await sleep(STEP_MS);
      if (code !== '123456') {
        listeners.forEach(l => l.authError('invalid_code'));
        throw new CoreError.SignIn({ code: 'invalid_code', detail: 'wrong code' });
      }
      signedIn('0xemail000000000000000000000000000000000002');
    },
    signInBrowser: () => {
      needSignedOut();
      emit(new State.SigningIn({ method: SignInMethod.Browser, step: SignInStep.AwaitingBrowser }));
    },
    completeBrowser: async () => {
      await sleep(STEP_MS);
      signedIn('0xbrowser0000000000000000000000000000000003');
    },
    cancelSignIn: () => emit(new State.SignedOut()),
    signInWalletBegin: () => {
      needSignedOut();
      emit(new State.SigningIn({ method: SignInMethod.Wallet, step: SignInStep.AwaitingWallet }));
      return 'Mock message to sign';
    },
    signInWalletComplete: () => signedIn('0xwallet000000000000000000000000000000000004'),
    handleDeepLink: url => {
      const id = /[?&]signin=([^&]+)/.exec(url)?.[1];
      if (id) {
        signedIn('0xbrowser0000000000000000000000000000000003');
      }
      return { navigation: undefined, effects: [] };
    },
    signOut: () => {
      wallet = undefined;
      emit(new State.SignedOut());
    },
  };
}
