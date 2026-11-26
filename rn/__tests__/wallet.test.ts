import { plainAddress, reason, toHex, WalletError } from '../src/flow/wallet';

describe('wallet sign-in helpers', () => {
  it('hex-encodes the message as UTF-8 for personal_sign', () => {
    expect(toHex('Decentraland Login\nÉ')).toBe(
      '0x446563656e7472616c616e64204c6f67696e0ac389',
    );
  });

  it('takes the address out of a CAIP-10 account', () => {
    const address = '0x1234567890abcdef1234567890ABCDEF12345678';
    expect(plainAddress(`eip155:1:${address}`)).toBe(address);
  });

  it('refuses an account that is not an Ethereum address', () => {
    expect(() => plainAddress(undefined)).toThrow(WalletError);
    expect(() => plainAddress('eip155:1:0x12')).toThrow(WalletError);
  });

  it('reads a refusal in the wallet from its code or message', () => {
    expect(
      reason({ code: 4001, message: 'User denied message signature' }),
    ).toBe('rejected');
    expect(reason({ code: 5000, message: 'User rejected.' })).toBe('rejected');
    expect(reason(new Error('Proposal expired'))).toBe('failed');
    expect(reason(undefined)).toBe('failed');
  });
});
