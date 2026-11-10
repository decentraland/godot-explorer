import { registerPreview } from '../gallery/registry';
import { createMockApi } from '../backpack/mockApi';
import { ApiProvider } from '../discover/ApiContext';
import { FtueScreen } from '../discover/FtueScreen';
import { createMockApi as createDiscoverApi } from '../discover/mockApi';
import { PLACES } from '../discover/mockData';
import { CustomizeStep } from './CustomizeStep';
import { AvatarCreateScreen, StageImage } from './previewScreens';
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
registerPreview('extra/flow-create', {
  title: 'Flow: avatar create',
  orientation: 'portrait',
  notes:
    'Figma AVATAR_CREATE 2776:23661; the presets are placeholders and no avatar shows.',
  render: () => (
    <AvatarCreateScreen
      selected={0}
      onSelect={noop}
      onCustomize={noop}
      onContinue={noop}
      onBack={noop}
    />
  ),
});
registerPreview('extra/flow-naming', {
  title: 'Flow: avatar naming',
  orientation: 'portrait',
  notes: 'Figma AVATAR_NAMING 2776:22117; the stage stands in for the avatar.',
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
      'Figma AVATAR_CUSTOMIZE 2760:15896, opened on Face',
    ],
    [
      'extra/flow-customize-finish',
      'finish',
      'Figma 2757:17247: DONE asks to finish the look',
    ],
    [
      'extra/flow-customize-discard',
      'discard',
      'Figma 2757:17014: back asks to discard',
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

const ftueApi = {
  ...createDiscoverApi(),
  ftuePlaces: async () => PLACES.slice(0, 4),
};
registerPreview('extra/flow-ftue', {
  title: 'Flow: Discover FTUE',
  orientation: 'portrait',
  notes: 'Figma DISCOVER_FTUE 1458:8149; the places are mock data.',
  render: () => (
    <ApiProvider api={ftueApi}>
      <FtueScreen username="Ana" onJump={noop} onSkip={noop} />
    </ApiProvider>
  ),
});
