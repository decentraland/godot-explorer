import { registerPreview } from '../gallery/registry';
import { createMockApi } from '../backpack/mockApi';
import { ApiProvider } from '../discover/ApiContext';
import { FtueScreen } from '../discover/FtueScreen';
import { createMockApi as createDiscoverApi } from '../discover/mockApi';
import { PLACES } from '../discover/mockData';
import { AvatarCustomize } from './AvatarCustomize';
import { AuthOtpEntercode } from './AuthOtpEntercode';
import { UpdateAvailable } from './UpdateAvailable';
import { AvatarCreate, StageImage, Comeback } from './previewScreens';
import {
  AccountHome,
  AuthOtpStart,
  AuthVerify,
  AccountGuestCreate,
  AvatarNaming,
  AuthBrowserOpen,
  AuthHome,
  DclSplash,
} from './screens';

const noop = () => {};

// Shown after sign out / delete account (settings-portrait); the entry-flow screens are the same.
['738-8292', '738-8297'].forEach(node =>
  registerPreview(`settings-portrait__dcl-splash__${node}`, {
    title: 'Flow: splash',
    orientation: 'portrait',
    notes: 'Entry-flow splash; the Godot splash is native',
    render: () => <DclSplash onDone={noop} />,
  }),
);
registerPreview('extra/flow-create', {
  title: 'Flow: avatar create',
  orientation: 'portrait',
  notes:
    'Figma AVATAR_CREATE 2776:23661; the presets are placeholders and no avatar shows.',
  render: () => (
    <AvatarCreate
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
    <AvatarNaming
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
      <AvatarCustomize
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
  render: () => <AccountGuestCreate />,
});
['738-8278', '738-8261'].forEach(node =>
  registerPreview(`settings-portrait__account-home__${node}`, {
    title: 'Flow: account home',
    orientation: 'portrait',
    notes: 'Entry-flow account screen shown after sign out or delete account',
    render: () => <AccountHome onGuest={noop} onSignIn={noop} />,
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

registerPreview('extra/flow-signin', {
  title: 'Flow: sign in providers',
  orientation: 'portrait',
  notes: 'Figma AUTH_HOME_ANDROID // IOS 1842:13740.',
  render: () => <AuthHome error={null} onProvider={noop} onBack={noop} />,
});

registerPreview('extra/flow-email', {
  title: 'Flow: email',
  orientation: 'portrait',
  notes: 'Figma AUTH_OTP_START 1842:13788.',
  render: () => (
    <AuthOtpStart
      initial=""
      error={null}
      busy={false}
      onSubmit={noop}
      onBack={noop}
    />
  ),
});
(
  [
    ['extra/flow-code', 'AUTH_OTP_ENTERCODE 1842:14149', null, false],
    ['extra/flow-code-verifying', 'AUTH_OTP_VERIFY 1842:14336', null, true],
    [
      'extra/flow-code-error',
      'AUTH_OTP_ERROR 1842:14452',
      'The code is invalid or expired. Please resend code.',
      false,
    ],
  ] as const
).forEach(([id, frame, error, busy]) =>
  registerPreview(id, {
    title: `Flow: code (${frame.split(' ')[0]})`,
    orientation: 'portrait',
    notes: `Figma ${frame}; the code is asked over the email screen.`,
    render: () => (
      <>
        <AuthOtpStart
          initial=""
          error={null}
          busy={false}
          onSubmit={noop}
          onBack={noop}
        />
        <AuthOtpEntercode
          visible
          email="your@email.com"
          error={error}
          busy={busy}
          onSubmit={noop}
          onResend={noop}
          onClose={noop}
          initialCode={busy || error ? '111111' : ''}
        />
      </>
    ),
  }),
);

registerPreview('extra/flow-code-resend-wait', {
  title: 'Flow: code (Resend Code waiting)',
  orientation: 'portrait',
  notes: 'Figma note: the link text is disabled for 2 minutes after each send.',
  render: () => (
    <>
      <AuthOtpStart
        initial=""
        error={null}
        busy={false}
        onSubmit={noop}
        onBack={noop}
      />
      <AuthOtpEntercode
        visible
        email="your@email.com"
        error={null}
        busy={false}
        onSubmit={noop}
        onResend={noop}
        onClose={noop}
        resendAt={Date.now() + 120000}
      />
    </>
  ),
});

registerPreview('extra/flow-comeback', {
  title: 'Flow: welcome back',
  orientation: 'portrait',
  notes: 'Figma COMEBACK 2776:23616; no avatar shows here.',
  render: () => (
    <Comeback name="Username" onContinue={noop} onUseAnother={noop} />
  ),
});

registerPreview('extra/flow-auth-browser-open', {
  title: 'Flow: AUTH_BROWSER_OPEN',
  orientation: 'portrait',
  notes: 'Figma AUTH_BROWSER_OPEN 1564:9629.',
  render: () => <AuthBrowserOpen onCancel={noop} />,
});
registerPreview('extra/flow-auth-browser-loading', {
  title: 'Flow: AUTH_BROWSER_OPEN (back from the browser)',
  orientation: 'portrait',
  notes: 'Figma AUTH_BROWSER_OPEN 2582:15409.',
  render: () => <AuthBrowserOpen loading onCancel={noop} />,
});
registerPreview('extra/flow-auth-verify', {
  title: 'Flow: AUTH_VERIFY',
  orientation: 'portrait',
  notes: 'Figma AUTH_VERIFY / AUTH_SUCCESS / AUTH_ERROR 1842:14702.',
  render: () => <AuthVerify />,
});
registerPreview('extra/flow-update', {
  title: 'Flow: Update available',
  orientation: 'portrait',
  notes: 'Figma Update available 1546:14269.',
  render: () => <UpdateAvailable required={false} onLater={noop} />,
});
registerPreview('extra/flow-update-forced', {
  title: 'Flow: Update available (forced)',
  orientation: 'portrait',
  notes: 'Figma Update available 1546:14186.',
  render: () => <UpdateAvailable required onLater={noop} />,
});
