import { Provider } from './flowMachine';

export type AuthResult = { hasProfile: boolean; name: string };

const WAIT_MS = 1800;

/**
 * Spike stand-in for the real sign-in (thirdweb / browser hand-off / wallet), which lives in the
 * Rust client and is not reachable while Godot only renders the avatar. Wallet resolves to a
 * returning account so both lobby branches can be walked through.
 */
export function signIn(provider: Provider): Promise<AuthResult> {
  return new Promise(resolve => {
    setTimeout(
      () =>
        resolve(
          provider === 'wallet'
            ? { hasProfile: true, name: 'Explorer' }
            : { hasProfile: false, name: '' },
        ),
      WAIT_MS,
    );
  });
}

export function startGuestSession(): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, WAIT_MS));
}
