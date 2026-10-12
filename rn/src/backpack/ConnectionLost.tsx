import { StyleSheet, View } from 'react-native';
import { SvgXml } from 'react-native-svg';
import { AppText, PrimaryButton } from '../ds';
import { u } from '../ds/scale';

const WIFI_OFF = `<svg viewBox="0 0 70 70" fill="none"><rect x="1.5" y="1.5" width="67" height="67" rx="10" stroke="#CFCDD4" stroke-width="3"/><path d="M14 27 Q35 10 56 27 M22 35 Q35 25 48 35 M29 42 Q35 38 41 42" stroke="#CFCDD4" stroke-width="3" stroke-linecap="round"/><circle cx="35" cy="48" r="3" fill="#CFCDD4"/><path d="M16 14 L54 44" stroke="#CFCDD4" stroke-width="3" stroke-linecap="round"/><path d="M19 57 H25 M31 57 H51" stroke="#CFCDD4" stroke-width="3" stroke-linecap="round"/></svg>`;

/** Figma "Lost connection" state shown when the catalog can't load. */
export function ConnectionLost({ onRetry }: { onRetry: () => void }) {
  return (
    <View style={styles.root}>
      <SvgXml xml={WIFI_OFF} width={70} height={70} />
      <AppText variant="heading" style={styles.title}>
        Connection lost
      </AppText>
      <AppText variant="body" style={styles.body}>
        Please check your internet connection and try again.
      </AppText>
      <View style={styles.action}>
        <PrimaryButton label="Try again" tone="secondary" onPress={onRetry} />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: u(32),
  },
  title: { marginTop: u(24), fontSize: u(24), color: '#E3D3F0' },
  body: { marginTop: u(8), textAlign: 'center', opacity: 0.8 },
  action: { marginTop: u(20) },
});
