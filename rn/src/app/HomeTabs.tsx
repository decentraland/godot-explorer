import { StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { BackpackScreen } from '../backpack/BackpackScreen';
import { DiscoverScreen } from '../discover/DiscoverScreen';
import { AppText, BottomNav, colors } from '../ds';
import { u } from '../ds/scale';
import { I18nKey, t } from '../i18n';
import { useNav } from './NavProvider';
import { getScreen } from './registry';
import type { Place } from '../discover/api';

const TITLES: Record<'settings' | 'profile', I18nKey> = {
  settings: 'SETTINGS_SETTINGS',
  profile: 'PROFILE_PROFILE',
};

/** Portrait tab shell. Settings and profile render the registered screen, else a stub. */
export function HomeTabs({ onJumpIn }: { onJumpIn: (place: Place) => void }) {
  const { state, setTab } = useNav();
  const insets = useSafeAreaInsets();
  const tab = state.tab;
  const footer = (
    <BottomNav active={tab} bottomInset={insets.bottom} onChange={setTab} />
  );

  if (tab === 'backpack') {
    return <BackpackScreen footer={footer} />;
  }
  if (tab === 'explore') {
    return <DiscoverScreen footer={footer} onJumpIn={onJumpIn} />;
  }
  const Registered = getScreen(tab);
  if (Registered) {
    return <Registered params={undefined as never} footer={footer} />;
  }
  return (
    <View style={styles.root}>
      <View style={styles.body}>
        <AppText variant="heading">{t(TITLES[tab])}</AppText>
        <AppText variant="body" style={styles.copy}>
          {t('APP_COMING_SOON')}
        </AppText>
      </View>
      {footer}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple550 },
  body: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  copy: { marginTop: u(8), opacity: 0.7 },
});
