import { useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { BackpackScreen } from './backpack/BackpackScreen';
import { AppText, BottomNav, colors, NavKey } from './ds';
import { Place } from './discover/api';
import { DiscoverScreen } from './discover/DiscoverScreen';
import { u } from './ds/scale';

/** Signed-in shell: bottom navigation with the Backpack as the one finished destination. */
export function Home({ onJumpIn }: { onJumpIn: (place: Place) => void }) {
  const [tab, setTab] = useState<NavKey>('explore');
  const insets = useSafeAreaInsets();
  const nav = (
    <BottomNav active={tab} bottomInset={insets.bottom} onChange={setTab} />
  );

  if (tab === 'backpack') {
    return <BackpackScreen footer={nav} />;
  }
  if (tab === 'explore') {
    return <DiscoverScreen footer={nav} onJumpIn={onJumpIn} />;
  }
  return (
    <View style={styles.root}>
      <View style={styles.body}>
        <AppText variant="heading">{TITLES[tab]}</AppText>
        <AppText variant="body" style={styles.copy}>
          Not part of this spike yet.
        </AppText>
      </View>
      {nav}
    </View>
  );
}

const TITLES: Record<NavKey, string> = {
  explore: 'Discover',
  backpack: 'Backpack',
  settings: 'Settings',
  profile: 'Profile',
};

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple550 },
  body: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  copy: { marginTop: u(8), opacity: 0.7 },
});
