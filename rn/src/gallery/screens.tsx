// Previews of the screens that exist today. Feature packages add their own from their dir with
// `registerPreview(manifestId, ...)` and one import line here.
import '../backpack/gallery';
import '../discover/gallery';
import '../flow/gallery';
import '../ftue/gallery';
import { AccountScreen, TermsScreen, WaitScreen } from '../flow/screens';
import '../hud/gallery';
import '../menu/gallery';
import '../profile/gallery';
import '../settings/gallery';
import { StyleSheet, View } from 'react-native';
import { BottomNav, colors, Menu } from '@dcl/ui-native';
import './dsButtons';
import './dsCards';
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
registerPreview('extra/nav-bar', {
  title: 'NavBar: portrait bar',
  orientation: 'portrait',
  render: () => (
    <View style={styles.navBar}>
      <BottomNav
        active="settings"
        badges={{ explore: true }}
        bottomInset={0}
        onChange={noop}
      />
    </View>
  ),
});
registerPreview('extra/menu', {
  title: 'Menu: floating context menu',
  orientation: 'portrait',
  render: () => (
    <View style={styles.menu}>
      <Menu
        items={[
          { key: 'a', label: 'Text', icon: 'location' },
          { key: 'b', label: 'Text', icon: 'location', selected: true },
          { key: 'c', label: 'Text', icon: 'location', check: true },
        ]}
        onSelect={noop}
      />
    </View>
  ),
});
registerPreview('extra/nav-rail', {
  title: 'NavBar: landscape rail',
  orientation: 'landscape',
  render: () => (
    <View style={styles.navRail}>
      <BottomNav
        landscape
        active="backpack"
        badges={{ explore: true }}
        onChange={noop}
      />
    </View>
  ),
});

const styles = StyleSheet.create({
  navBar: {
    flex: 1,
    justifyContent: 'flex-end',
    backgroundColor: colors.modal,
  },
  navRail: { flex: 1, flexDirection: 'row', backgroundColor: colors.modal },
  menu: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.modal,
  },
});
