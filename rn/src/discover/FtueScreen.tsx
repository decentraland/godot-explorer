import { useEffect, useState } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import {
  AppText,
  Button,
  colors,
  fonts,
  Icon,
  LinearFill,
  PageDots,
  SnapCarousel,
  u,
} from '@dcl/ui-native';
import { t } from '../i18n';
import { Place } from './api';
import { useApi } from './ApiContext';
import { ftuePlaces } from './ftue';
import { Thumb } from './Thumb';

const CARD_W = 312;
const CARD_H = 175;
/** The cards beside the settled one are 530 wide in the frame, against its 624. */
const SIDE_SCALE = 530 / 624;

const SHADE = [
  { offset: 0, color: colors.shadow, opacity: 0 },
  { offset: 1, color: colors.shadow, opacity: 0.9 },
];

function Card({ place }: { place: Place }) {
  return (
    <View style={styles.card}>
      <Thumb
        uri={place.image}
        width="100%"
        height="100%"
        radius={0}
        bordered={false}
      />
      <View style={styles.cardShade}>
        <LinearFill stops={SHADE} />
      </View>
      <View style={styles.cardName}>
        <AppText numberOfLines={2} style={styles.cardNameText}>
          {place.title}
        </AppText>
      </View>
    </View>
  );
}

/**
 * Figma `DISCOVER_FTUE`: the `ftue` places as a snap carousel with the name over each card, JUMP IN
 * for the settled one and Explore more to Discover. Without `places` it fills when the list lands.
 */
export function FtueScreen({
  username,
  places,
  initialIndex = 0,
  onJump,
  onSkip,
}: {
  username: string;
  places?: Place[];
  initialIndex?: number;
  onJump: (place: Place) => void;
  onSkip: () => void;
}) {
  const api = useApi();
  const [loaded, setLoaded] = useState<Place[]>([]);
  const [index, setIndex] = useState(initialIndex);
  useEffect(() => {
    if (!places) {
      ftuePlaces(api).then(setLoaded);
    }
  }, [api, places]);

  const list = places ?? loaded;
  const current = list[Math.min(index, list.length - 1)];

  return (
    <View style={styles.root}>
      <View style={styles.headline}>
        <AppText style={styles.welcome}>
          {t('FTUE_WELCOME_TITLE')}{' '}
          <AppText style={[styles.welcome, styles.name]}>@{username}</AppText>
        </AppText>
        <AppText style={styles.welcome}>{t('FTUE_GET_STARTED')}</AppText>
      </View>
      <View style={styles.recommended}>
        <AppText style={styles.label}>{t('FTUE_RECOMMENDED_PLACES')}</AppText>
        {current ? (
          <>
            {list.length === 1 ? (
              <View style={styles.single}>
                <Card place={current} />
              </View>
            ) : (
              <View style={styles.carousel}>
                <SnapCarousel
                  data={list}
                  itemWidth={CARD_W}
                  gap={8}
                  inset={24}
                  initialIndex={initialIndex}
                  sideScale={SIDE_SCALE}
                  keyOf={place => place.id}
                  onIndexChange={setIndex}
                  renderItem={place => <Card place={place} />}
                />
              </View>
            )}
            <View style={styles.dots}>
              <PageDots count={list.length} index={index} round dark />
            </View>
            <View style={styles.jump}>
              <Button
                big
                fullWidth
                label={t('COMMON_JUMP_IN')}
                trailingIcon="jump"
                onPress={() => onJump(current)}
              />
            </View>
          </>
        ) : null}
      </View>
      <Pressable
        accessibilityRole="button"
        hitSlop={12}
        onPress={onSkip}
        style={styles.explore}
      >
        <Icon name="compass" size={28} color={colors.white70} />
        <AppText style={styles.exploreText}>{t('FTUE_EXPLORE_MORE')}</AppText>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    ...StyleSheet.absoluteFill,
    backgroundColor: colors.brand,
    paddingTop: u(99),
    paddingBottom: u(36),
    gap: u(100),
  },
  headline: { alignItems: 'center' },
  welcome: {
    fontFamily: fonts.bold,
    fontSize: u(24),
    lineHeight: u(29),
    color: colors.snow,
    textAlign: 'center',
  },
  name: { color: colors.lavender },
  recommended: { alignSelf: 'stretch' },
  label: {
    marginLeft: u(24),
    fontFamily: fonts.bold,
    fontSize: u(12),
    lineHeight: u(14.5),
    color: colors.white70,
    textTransform: 'uppercase',
  },
  carousel: { marginTop: u(12.5) },
  single: { marginTop: u(12.5), alignItems: 'center' },
  dots: { marginTop: u(8) },
  jump: { marginTop: u(48), paddingHorizontal: u(24) },
  card: {
    width: u(CARD_W),
    height: u(CARD_H),
    borderRadius: u(4),
    borderWidth: 1,
    borderColor: colors.lavender,
    overflow: 'hidden',
    backgroundColor: colors.purple450,
  },
  cardShade: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: u(84.5),
  },
  cardName: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    paddingHorizontal: u(16),
    paddingVertical: u(15),
  },
  cardNameText: {
    fontFamily: fonts.semibold,
    fontSize: u(13),
    lineHeight: u(15.5),
    color: colors.snow,
  },
  explore: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(4),
  },
  exploreText: {
    fontFamily: fonts.bold,
    fontSize: u(15),
    lineHeight: u(18),
    color: colors.white70,
  },
});
