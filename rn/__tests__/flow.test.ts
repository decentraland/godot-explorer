import {
  FlowAction,
  FlowState,
  initialFlow,
  isValidName,
  reduceFlow,
} from '../src/flow/flowMachine';

const run = (actions: FlowAction[], from: FlowState = initialFlow) =>
  actions.reduce(reduceFlow, from);

describe('entry flow', () => {
  it('goes splash → terms → account', () => {
    expect(run([{ type: 'booted' }]).step).toBe('terms');
    expect(run([{ type: 'booted' }, { type: 'acceptTerms' }]).step).toBe(
      'account',
    );
  });

  it('guest path skips naming and ends in the ftue', () => {
    const state = run([
      { type: 'booted' },
      { type: 'acceptTerms' },
      { type: 'playAsGuest' },
      { type: 'guestReady' },
      { type: 'customizeAvatar' },
      { type: 'avatarConfirmed' },
    ]);
    expect(state.step).toBe('ftue');
    expect(state.guest).toBe(true);
    expect(run([{ type: 'finish' }], state).step).toBe('done');
  });

  it('new account goes through avatar and naming', () => {
    const state = run([
      { type: 'booted' },
      { type: 'acceptTerms' },
      { type: 'chooseSignIn' },
      { type: 'chooseProvider', provider: 'google' },
      { type: 'authSucceeded', hasProfile: false, name: '' },
    ]);
    expect(state.step).toBe('avatar_create');
    const named = run(
      [{ type: 'avatarConfirmed' }, { type: 'named', name: 'Mateo' }],
      state,
    );
    expect(named.step).toBe('ftue');
    expect(named.name).toBe('Mateo');
  });

  it('returning account sees welcome back with its name', () => {
    const state = run([
      { type: 'chooseSignIn' },
      { type: 'chooseProvider', provider: 'wallet' },
      { type: 'authSucceeded', hasProfile: true, name: 'Ana' },
    ]);
    expect(state.step).toBe('welcome_back');
    expect(state.name).toBe('Ana');
  });

  it('returns to sign-in with a message when auth fails, and ignores stale auth results', () => {
    const waiting = run([
      { type: 'chooseSignIn' },
      { type: 'chooseProvider', provider: 'apple' },
    ]);
    const failed = reduceFlow(waiting, { type: 'authFailed', message: 'nope' });
    expect(failed.step).toBe('sign_in');
    expect(failed.error).toBe('nope');
    const cancelled = reduceFlow(waiting, { type: 'back' });
    expect(cancelled.step).toBe('sign_in');
    expect(
      reduceFlow(cancelled, {
        type: 'authSucceeded',
        hasProfile: true,
        name: 'x',
      }).step,
    ).toBe('sign_in');
  });

  it('email goes through the address, then the code, then succeeds', () => {
    const entry = run([
      { type: 'chooseSignIn' },
      { type: 'chooseProvider', provider: 'email' },
    ]);
    expect(entry.step).toBe('email_entry');
    const code = reduceFlow(entry, { type: 'emailSent', email: 'a@b.co' });
    expect(code.step).toBe('email_code');
    expect(code.email).toBe('a@b.co');
    const done = reduceFlow(code, {
      type: 'authSucceeded',
      hasProfile: true,
      name: 'Ana',
    });
    expect(done.step).toBe('welcome_back');
  });

  it('back from the code returns to the address, and a late code result is ignored', () => {
    const code = run([
      { type: 'chooseSignIn' },
      { type: 'chooseProvider', provider: 'email' },
      { type: 'emailSent', email: 'a@b.co' },
    ]);
    const entry = reduceFlow(code, { type: 'back' });
    expect(entry.step).toBe('email_entry');
    expect(reduceFlow(entry, { type: 'back' }).step).toBe('sign_in');
    expect(
      reduceFlow(entry, { type: 'authSucceeded', hasProfile: false, name: '' })
        .step,
    ).toBe('email_entry');
    expect(
      reduceFlow(initialFlow, { type: 'emailSent', email: 'x@y.zz' }).step,
    ).toBe('splash');
  });

  it('back walks up the tree and never mutates', () => {
    const state = run([{ type: 'chooseSignIn' }]);
    expect(reduceFlow(state, { type: 'back' }).step).toBe('account');
    expect(state.step).toBe('sign_in');
    expect(reduceFlow(initialFlow, { type: 'back' })).toBe(initialFlow);
  });
});

describe('isValidName', () => {
  it('accepts 3–15 alphanumerics only', () => {
    expect(isValidName('Mateo')).toBe(true);
    expect(isValidName('  Ana3 ')).toBe(true);
    expect(isValidName('ab')).toBe(false);
    expect(isValidName('has space')).toBe(false);
    expect(isValidName('a'.repeat(16))).toBe(false);
  });
});
