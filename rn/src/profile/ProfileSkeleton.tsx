import { StyleSheet, View } from 'react-native';
import { Skeleton } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';

/** Placeholder for the name, address, button and About rows while the profile loads. */
export function ProfileSkeleton() {
  return (
    <View accessibilityLabel={t('APP_LOADING_PROFILE')}>
      <Skeleton width="60%" height={26} radius={6} />
      <Skeleton width="38%" height={16} style={{ marginTop: u(10) }} />
      <Skeleton width="100%" height={48} style={{ marginTop: u(26) }} />
      <Skeleton width="30%" height={14} style={{ marginTop: u(30) }} />
      <Skeleton width="100%" height={52} style={{ marginTop: u(10) }} />
      {[0, 1, 2].map(i => (
        <View key={i} style={styles.row}>
          <Skeleton width="45%" height={30} />
          <Skeleton width="45%" height={30} />
        </View>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', marginTop: u(16), gap: u(24) },
});
