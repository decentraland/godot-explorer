import { Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, Icon } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';

export function DiscoverHeader({
  title,
  onBack,
  onSearch,
  searchDisabled = false,
}: {
  title: string;
  onBack?: () => void;
  onSearch: () => void;
  searchDisabled?: boolean;
}) {
  const insets = useSafeAreaInsets();
  return (
    <View style={[styles.header, { paddingTop: insets.top + u(6) }]}>
      {onBack ? (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('DISCOVER_BACK')}
          hitSlop={12}
          onPress={onBack}
          style={styles.back}
        >
          <Icon name="chevronLeft" size={22} strokeWidth={2.2} />
        </Pressable>
      ) : null}
      <AppText variant="heading" style={styles.title}>
        {title}
      </AppText>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('DISCOVER_SEARCH')}
        accessibilityState={{ disabled: searchDisabled }}
        disabled={searchDisabled}
        hitSlop={12}
        onPress={onSearch}
        style={searchDisabled && styles.disabled}
      >
        <Icon name="search" size={24} strokeWidth={2.2} />
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: u(24),
    paddingBottom: u(12),
  },
  back: { marginRight: u(12) },
  title: { flex: 1, fontSize: u(24) },
  disabled: { opacity: 0.4 },
});
