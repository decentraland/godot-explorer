/** Entry flow mirroring the Godot lobby (godot/src/ui/pages/auth/lobby.gd), portrait only. */
export type Step =
  | 'splash'
  | 'terms'
  | 'account'
  | 'sign_in'
  | 'auth_wait'
  | 'email_entry'
  | 'email_code'
  | 'guest_wait'
  | 'welcome_back'
  | 'avatar_create'
  | 'avatar_customize'
  | 'avatar_naming'
  | 'ftue'
  | 'done';

export type Provider = 'google' | 'apple' | 'email' | 'wallet';

export type FlowState = {
  step: Step;
  guest: boolean;
  provider: Provider | null;
  email: string;
  name: string;
  error: string | null;
};

export type FlowAction =
  | { type: 'booted' }
  | { type: 'acceptTerms' }
  | { type: 'playAsGuest' }
  | { type: 'guestReady' }
  | { type: 'chooseSignIn' }
  | { type: 'chooseProvider'; provider: Provider }
  | { type: 'emailSent'; email: string }
  | { type: 'authSucceeded'; hasProfile: boolean; name: string }
  | { type: 'authFailed'; message: string }
  | { type: 'customizeAvatar' }
  | { type: 'avatarConfirmed' }
  | { type: 'named'; name: string }
  | { type: 'finish' }
  | { type: 'back' };

export const initialFlow: FlowState = {
  step: 'splash',
  guest: false,
  provider: null,
  email: '',
  name: '',
  error: null,
};

const go = (
  state: FlowState,
  step: Step,
  patch: Partial<FlowState> = {},
): FlowState => ({
  ...state,
  error: null,
  ...patch,
  step,
});

export function reduceFlow(state: FlowState, action: FlowAction): FlowState {
  switch (action.type) {
    case 'booted':
      return state.step === 'splash' ? go(state, 'terms') : state;
    case 'acceptTerms':
      return go(state, 'account');
    case 'playAsGuest':
      return go(state, 'guest_wait', { guest: true, provider: null });
    case 'guestReady':
      return state.step === 'guest_wait' ? go(state, 'avatar_create') : state;
    case 'chooseSignIn':
      return go(state, 'sign_in', { guest: false });
    case 'chooseProvider':
      return action.provider === 'email'
        ? go(state, 'email_entry', { provider: action.provider })
        : go(state, 'auth_wait', { provider: action.provider });
    case 'emailSent':
      return state.step === 'email_entry'
        ? go(state, 'email_code', { email: action.email })
        : state;
    case 'authSucceeded':
      if (state.step !== 'auth_wait' && state.step !== 'email_code') {
        return state;
      }
      return action.hasProfile
        ? go(state, 'welcome_back', { name: action.name })
        : go(state, 'avatar_create');
    case 'authFailed':
      return state.step === 'auth_wait' || state.step === 'email_entry'
        ? go(state, 'sign_in', { error: action.message })
        : state;
    case 'customizeAvatar':
      return go(state, 'avatar_customize');
    case 'avatarConfirmed':
      // Guests never pick a name (as in the lobby); accounts do.
      return go(state, state.guest ? 'ftue' : 'avatar_naming');
    case 'named':
      return go(state, 'ftue', { name: action.name });
    case 'finish':
      return go(state, 'done');
    case 'back':
      return back(state);
  }
}

function back(state: FlowState): FlowState {
  switch (state.step) {
    case 'account':
      return go(state, 'terms');
    case 'sign_in':
      return go(state, 'account');
    case 'auth_wait':
    case 'email_entry':
      return go(state, 'sign_in');
    case 'email_code':
      return go(state, 'email_entry');
    case 'guest_wait':
      return go(state, 'account', { guest: false });
    case 'avatar_create':
      return go(state, state.guest ? 'account' : 'sign_in', { guest: false });
    case 'avatar_customize':
      return go(state, 'avatar_create');
    case 'avatar_naming':
      return go(state, 'avatar_create');
    default:
      return state;
  }
}

/** Steps where the Godot avatar preview is on screen. */
export const PREVIEW_STEPS: readonly Step[] = [
  'avatar_create',
  'avatar_customize',
  'avatar_naming',
  'welcome_back',
];

/** Name rule from the lobby naming screen: 3–15 letters or digits. */
export const NAME_PATTERN = /^[A-Za-z0-9]{3,15}$/;

export const isValidName = (name: string) => NAME_PATTERN.test(name.trim());
