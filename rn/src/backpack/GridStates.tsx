import { Pressable, StyleSheet, View } from 'react-native';
import Svg, { Path } from 'react-native-svg';
import { AppText, colors, Icon, IconName, SkeletonCard, layout } from '../ds';
import { t } from '../i18n';
import type { EmptyCopy } from './platformPolicy';
import { u } from '../ds/scale';

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

/** "Nothing here yet" with the store-policy hint: Marketplace link, plain Shop line, or no hint. */
export function EmptyBlock({
  icon,
  copy,
  onMarketplace,
}: {
  icon: IconName;
  copy: EmptyCopy;
  onMarketplace: () => void;
}) {
  return (
    <View style={[styles.empty, copy === 'none' && styles.emptyCentered]}>
      <View style={styles.iconBox}>
        {icon === 'wearables' ? (
          <Svg width={u(38)} height={u(38)} viewBox="0 0 24 24" fill="none">
            <Path
              d="M8 3.5 L2.5 6.5 L4.5 10.5 L7.5 9.5 V18 H16.5 V9.5 L19.5 10.5 L21.5 6.5 L16 3.5 Q12 6.5 8 3.5 Z M8 21 H11 M13.5 21 H16"
              stroke={colors.snow}
              strokeWidth={1.3}
              strokeLinejoin="round"
              strokeLinecap="round"
            />
          </Svg>
        ) : (
          <Icon name={icon} size={38} color={colors.snow} strokeWidth={1.6} />
        )}
      </View>
      <AppText variant="title" style={styles.title}>
        {t('BACKPACK_NOTHING_HERE_YET')}
      </AppText>
      {copy === 'marketplace_link' && (
        <>
          <AppText variant="body" style={styles.hint}>
            {t('BACKPACK_FIND_NEW_ONES_IN_THE')}
          </AppText>
          <Pressable
            accessibilityRole="link"
            onPress={onMarketplace}
            hitSlop={8}
          >
            <AppText variant="body" style={styles.link}>
              {t('BACKPACK_MARKETPLACE')}
            </AppText>
          </Pressable>
        </>
      )}
      {copy === 'shop' && (
        <AppText variant="body" style={styles.hint}>
          {t('BACKPACK_FIND_NEW_ONES_IN_THE_SHOP')}
        </AppText>
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
  empty: { alignItems: 'center', paddingTop: u(40), paddingHorizontal: u(24) },
  emptyCentered: { paddingTop: u(52) },
  iconBox: {
    width: u(60),
    height: u(60),
    borderRadius: u(8),
    borderWidth: 1.5,
    borderColor: colors.snow,
    alignItems: 'center',
    justifyContent: 'center',
  },
  title: { marginTop: u(14), fontSize: u(18), fontFamily: 'Inter-Bold' },
  hint: { marginTop: u(6), fontSize: u(14), textAlign: 'center' },
  link: {
    fontSize: u(14),
    color: '#A0ABFF',
    textDecorationLine: 'underline',
  },
});
