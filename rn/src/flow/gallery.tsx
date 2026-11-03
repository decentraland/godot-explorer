import { registerPreview } from '../gallery/registry';
import { AccountScreen, GuestCreateScreen, SplashScreen } from './screens';

const noop = () => {};

// Shown after sign out / delete account (settings-portrait); the entry-flow screens are the same.
['738-8292', '738-8297'].forEach(node =>
  registerPreview(`settings-portrait__dcl-splash__${node}`, {
    title: 'Flow: splash',
    orientation: 'portrait',
    notes: 'Entry-flow splash; the Godot splash is native',
    render: () => <SplashScreen onDone={noop} />,
  }),
);
registerPreview('extra/flow-guest-create', {
  title: 'Flow: guest create',
  orientation: 'portrait',
  notes: 'Entry flow: the guest account being created after Play as Guest',
  render: () => <GuestCreateScreen />,
});
['738-8278', '738-8261'].forEach(node =>
  registerPreview(`settings-portrait__account-home__${node}`, {
    title: 'Flow: account home',
    orientation: 'portrait',
    notes: 'Entry-flow account screen shown after sign out or delete account',
    render: () => <AccountScreen onGuest={noop} onSignIn={noop} />,
  }),
);
