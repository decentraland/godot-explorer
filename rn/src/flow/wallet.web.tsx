/** The web preview has no wallet apps to hand off to: a wallet sign-in fails at once. */
export type WalletKind = 'metamask' | 'wallet_connect';
export type WalletPhase = 'connecting' | 'signing';
export type WalletErrorCode =
  | 'cancelled'
  | 'not_installed'
  | 'rejected'
  | 'failed';

export class WalletError extends Error {
  readonly code: WalletErrorCode;
  constructor(code: WalletErrorCode) {
    super(code);
    this.code = code;
  }
}

export function WalletModal() {
  return null;
}

export async function signInWithWallet(
  _kind: WalletKind,
  _onPhase: (phase: WalletPhase) => void,
): Promise<void> {
  throw new WalletError('failed');
}

export function cancelWalletSignIn() {}
