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
  /** Launched by a preview link with no account: the guest login runs silently and skips the avatar. */
  preview: boolean;
  provider: Provider | null;
  email: string;
  name: string;
  error: string | null;
};

export type FlowAction =
  | { type: 'booted'; next?: 'terms' | 'account' }
  | { type: 'recovered'; hasProfile: boolean; name: string }
  | { type: 'acceptTerms' }
  | { type: 'playAsGuest' }
  | { type: 'previewGuest' }
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
  preview: false,
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
      return state.step === 'splash'
        ? go(state, action.next ?? 'terms')
        : state;
    case 'recovered':
      // A restored account that already has a name goes straight into the app, as the lobby did.
      if (state.step !== 'splash') {
        return state;
      }
      return action.hasProfile
        ? go(state, 'done', { name: action.name })
        : go(state, 'avatar_create');
    case 'acceptTerms':
      return go(state, 'account');
    case 'playAsGuest':
      return go(state, 'guest_wait', { guest: true, provider: null });
    case 'previewGuest':
      return state.step === 'splash'
        ? go(state, 'guest_wait', { guest: true, preview: true })
        : state;
    case 'guestReady':
      if (state.step !== 'guest_wait') {
        return state;
      }
      return go(state, state.preview ? 'done' : 'avatar_create');
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
      if (state.step === 'guest_wait' && state.preview) {
        // A preview that could not sign in is no longer honoured: the normal flow, with avatar creation.
        return go(state, 'account', { guest: false, preview: false });
      }
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
      return go(state, 'account', { guest: false, preview: false });
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
