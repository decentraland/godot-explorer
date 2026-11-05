import { registerPreview } from '../gallery/registry';
import { createMockApi } from '../backpack/mockApi';
import { CustomizeStep } from './CustomizeStep';
import { StageImage } from './previewScreens';
import {
  AccountScreen,
  GuestCreateScreen,
  NamingScreen,
  SplashScreen,
} from './screens';

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
registerPreview('extra/flow-naming', {
  title: 'Flow: avatar naming',
  orientation: 'portrait',
  notes: 'Figma AVATAR_NAMING 1897:13029; the stage stands in for the avatar.',
  render: () => (
    <NamingScreen
      initial="JuanPedro"
      tag="1234"
      preview={<StageImage />}
      onSubmit={noop}
      onBack={noop}
    />
  ),
});

const customizeApi = createMockApi();
(
  [
    [
      'extra/flow-customize',
      null,
      'Figma AVATAR_CUSTOMIZE 1574:10070, opened on Face',
    ],
    [
      'extra/flow-customize-finish',
      'finish',
      'Figma 2669:31655: DONE asks to finish the look',
    ],
    [
      'extra/flow-customize-discard',
      'discard',
      'Figma 1835:9418: back asks to discard',
    ],
  ] as const
).forEach(([id, prompt, notes]) =>
  registerPreview(id, {
    title: `Flow: customize${prompt ? ` (${prompt})` : ''}`,
    orientation: 'portrait',
    notes,
    render: () => (
      <CustomizeStep
        key={id}
        api={customizeApi}
        initialPrompt={prompt}
        onDone={noop}
        onDiscard={noop}
      />
    ),
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
