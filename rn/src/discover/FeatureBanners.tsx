import { useState } from 'react';
import { Animated, StyleSheet, View } from 'react-native';
import {
  Feature,
  FEATURE,
  PageDots,
  Skeleton,
  SnapCarousel,
  u,
} from '@dcl/ui-native';
import { mockArt } from './Thumb';
import type { Banner } from './types';

/** Pulls a side card in so its visible edge sits 8 dp from the settled one, as in Discover-FeatureBanners. */
const SIDE_SHIFT = u((FEATURE.width * (1 - FEATURE.defaultScaleX)) / 2);

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
          width={u(FEATURE.width)}
          height={FEATURE.height}
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
        itemWidth={FEATURE.width}
        gap={8}
        inset={24}
        keyOf={b => b.id}
        onIndexChange={setIndex}
        renderItem={(banner, i, position) => (
          <Animated.View
            style={{
              transform: [
                {
                  translateX: position.interpolate({
                    inputRange: [-1, 0, 1],
                    outputRange: [SIDE_SHIFT, 0, -SIDE_SHIFT],
                  }),
                },
              ],
            }}
          >
            <Feature
              state={i === index ? 'Selected' : 'Default'}
              transition={position.interpolate({
                inputRange: [-1, 0, 1],
                outputRange: [0, 1, 0],
              })}
              title={banner.title || banner.place.title}
              image={banner.image.startsWith('mock:') ? '' : banner.image}
              art={mockArt(banner.image)}
              onPress={() => onOpen(banner)}
              onJumpIn={() => onJump(banner)}
            />
          </Animated.View>
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
  dots: { marginTop: u(8) },
});
