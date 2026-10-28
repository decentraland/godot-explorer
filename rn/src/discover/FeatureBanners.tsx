import { useState } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import {
  AppText,
  colors,
  Icon,
  LinearFill,
  PageDots,
  Skeleton,
  SnapCarousel,
} from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { Thumb } from './Thumb';
import type { Banner } from './types';

const WIDTH = 312;
const HEIGHT = 174;
const SCRIM = [
  { offset: 0, color: '#000000', opacity: 0 },
  { offset: 1, color: '#000000', opacity: 0.75 },
];

function BannerCard({
  banner,
  onOpen,
  onJump,
}: {
  banner: Banner;
  onOpen: () => void;
  onJump: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={banner.title || banner.place.title}
      onPress={onOpen}
    >
      <Thumb
        uri={banner.image}
        width={u(WIDTH)}
        height={u(HEIGHT)}
        radius={6}
      />
      <View style={styles.scrim} pointerEvents="none">
        <LinearFill stops={SCRIM} angle={180} />
      </View>
      <View style={styles.border} pointerEvents="none" />
      {banner.cta === 'arrow' ? (
        <View style={styles.row}>
          <AppText variant="button" numberOfLines={2} style={styles.title}>
            {banner.title}
          </AppText>
          <View style={styles.arrow}>
            <Icon name="jump" size={16} strokeWidth={2.4} />
          </View>
        </View>
      ) : (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('COMMON_JUMP_IN')}
          onPress={onJump}
          style={styles.jump}
        >
          <AppText variant="button" style={styles.jumpText}>
            {t('COMMON_JUMP_IN')}
          </AppText>
        </Pressable>
      )}
    </Pressable>
  );
}

/** Snap carousel of featured places with a dot indicator; the skeleton is one banner-sized block. */
export function FeatureBanners({
  banners,
  loading,
  onOpen,
  onJump,
  initialIndex = 0,
}: {
  banners: Banner[] | null;
  loading: boolean;
  onOpen: (banner: Banner) => void;
  onJump: (banner: Banner) => void;
  initialIndex?: number;
}) {
  const [index, setIndex] = useState(initialIndex);
  if (loading) {
    return (
      <View style={styles.wrap}>
        <Skeleton
          width={u(WIDTH)}
          height={HEIGHT}
          radius={6}
          style={styles.skeleton}
        />
      </View>
    );
  }
  if (!banners || banners.length === 0) {
    return null;
  }
  return (
    <View style={styles.wrap}>
      <SnapCarousel
        data={banners}
        itemWidth={WIDTH}
        gap={8}
        inset={24}
        keyOf={b => b.id}
        onIndexChange={setIndex}
        renderItem={banner => (
          <BannerCard
            banner={banner}
            onOpen={() => onOpen(banner)}
            onJump={() => onJump(banner)}
          />
        )}
      />
      <View style={styles.dots}>
        <PageDots round count={banners.length} index={index} />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { marginTop: u(19) },
  skeleton: { marginLeft: u(24) },
  border: {
    ...StyleSheet.absoluteFill,
    borderRadius: u(6),
    borderWidth: 0.5,
    borderColor: colors.lavender,
  },
  scrim: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: u(HEIGHT * 0.5),
    borderBottomLeftRadius: u(6),
    borderBottomRightRadius: u(6),
    overflow: 'hidden',
  },
  row: {
    position: 'absolute',
    left: u(16),
    right: u(16),
    bottom: u(16),
    flexDirection: 'row',
    alignItems: 'flex-end',
    gap: u(12),
  },
  title: { flex: 1, fontSize: u(13), lineHeight: u(15.5), color: colors.snow },
  arrow: {
    width: u(28),
    height: u(28),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.ruby,
  },
  jump: {
    position: 'absolute',
    right: u(16),
    bottom: u(16),
    width: u(107),
    height: u(24),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.ruby,
  },
  jumpText: { fontFamily: 'Inter-Bold', fontSize: u(10) },
  dots: { marginTop: u(8) },
});
