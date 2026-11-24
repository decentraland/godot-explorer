import { StyleSheet, View } from 'react-native';
import {
  EmptyState,
  IconName,
  LinkButton,
  SkeletonCard,
  layout,
  space,
} from '@dcl/ui-native';
import { t } from '../i18n';
import type { EmptyCopy } from './platformPolicy';

export const SKELETON_CELLS = 12;

/** Twelve dim tiles standing in for the grid while the catalog loads (Figma "Skeleton"). */
export function SkeletonGrid({
  columns = layout.columns,
}: {
  columns?: number;
}) {
  return (
    <View style={styles.skeleton}>
      {Array.from({ length: SKELETON_CELLS }, (_, index) => (
        <View
          key={index}
          style={{ width: `${(100 - (columns - 1) * 4.2) / columns}%` }}
        >
          <SkeletonCard />
        </View>
      ))}
    </View>
  );
}

/** Figma `EmptyState` (lg) with the store-policy hint: Marketplace link, plain Shop line, or no hint. */
export function EmptyBlock({
  icon,
  copy,
  onMarketplace,
}: {
  icon: IconName;
  copy: EmptyCopy;
  onMarketplace: () => void;
}) {
  const body =
    copy === 'shop'
      ? t('BACKPACK_FIND_NEW_ONES_IN_THE_SHOP')
      : copy === 'marketplace_link'
      ? t('BACKPACK_FIND_NEW_ONES_IN_THE')
      : undefined;
  return (
    <View style={styles.empty}>
      <EmptyState
        icon={icon}
        title={t('BACKPACK_NOTHING_HERE_YET')}
        body={body}
      />
      {copy === 'marketplace_link' && (
        <LinkButton label={t('BACKPACK_MARKETPLACE')} onPress={onMarketplace} />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  skeleton: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    paddingHorizontal: layout.gutter,
    gap: layout.gap,
  },
  /** 42 px under the slot row in `Cards` (no 21dp token: `space['40']`). */
  empty: { alignItems: 'center', paddingTop: space['40'] },
});
