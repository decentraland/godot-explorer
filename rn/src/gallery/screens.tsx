// Previews of the screens that exist today. Feature packages add their own from their dir with
// `registerPreview(manifestId, ...)` and one import line here.
import '../backpack/gallery';
import '../discover/gallery';
import '../flow/gallery';
import { AccountScreen, TermsScreen, WaitScreen } from '../flow/screens';
import '../hud/gallery';
import '../menu/gallery';
import '../profile/gallery';
import '../settings/gallery';
import { registerPreview } from './registry';

const noop = () => {};

registerPreview('extra/flow-terms', {
  title: 'Flow: terms',
  orientation: 'portrait',
  render: () => <TermsScreen onAccept={noop} />,
});
registerPreview('extra/flow-account', {
  title: 'Flow: account',
  orientation: 'portrait',
  render: () => <AccountScreen onGuest={noop} onSignIn={noop} onBack={noop} />,
});
registerPreview('extra/flow-wait', {
  title: 'Flow: wait',
  orientation: 'portrait',
  render: () => (
    <WaitScreen title="Signing you in" body="One moment" onCancel={noop} />
  ),
});
