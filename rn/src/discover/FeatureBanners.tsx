import { useState } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import {
  AppText,
  colors,
  fonts,
  Icon,
  LinearFill,
  PageDots,
  Skeleton,
  SnapCarousel,
  u,
} from '@dcl/ui-native';
import { t } from '../i18n';
import { Thumb } from './Thumb';
import type { Banner } from './types';

const WIDTH = 312;
const HEIGHT = 175;
/** Figma `Feature` 819:3463 (Selected): Shadow 0 → 90% over the bottom 84.5 dp. */
const SCRIM = [
  { offset: 0, color: colors.shadow, opacity: 0 },
  { offset: 1, color: colors.shadow, opacity: 0.9 },
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
        radius={4}
      />
      <View style={styles.scrim} pointerEvents="none">
        <LinearFill stops={SCRIM} angle={180} />
      </View>
      <View style={styles.border} pointerEvents="none" />
      {banner.cta === 'arrow' ? (
        <View style={styles.row}>
          <AppText numberOfLines={2} style={styles.title}>
            {banner.title}
          </AppText>
          <View style={styles.arrow}>
            <Icon name="jump" size={12} />
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
          radius={4}
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
    borderRadius: u(4),
    borderWidth: 0.5,
    borderColor: colors.lavender,
  },
  scrim: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: u(84.5),
    borderBottomLeftRadius: u(4),
    borderBottomRightRadius: u(4),
    overflow: 'hidden',
  },
  row: {
    position: 'absolute',
    left: u(16),
    right: u(16),
    bottom: u(15),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(16),
  },
  title: {
    flex: 1,
    fontFamily: fonts.semibold,
    fontSize: u(13),
    color: colors.snow,
  },
  arrow: {
    width: u(27),
    height: u(27),
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
