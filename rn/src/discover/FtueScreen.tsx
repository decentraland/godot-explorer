import { useEffect, useState } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, SnapCarousel } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { Place } from './api';
import { useApi } from './ApiContext';
import { DiscoverBackground } from './DiscoverBackground';
import { ftuePlaces } from './ftue';
import { JumpGlyph } from './JumpGlyph';
import { Thumb } from './Thumb';

const CARD_W = 300;
const CARD_H = 240;
const STATIC_W = 275;

function Headline({ name }: { name: string }) {
  return (
    <View style={styles.headline}>
      <AppText variant="heading" style={styles.welcome}>
        {t('FTUE_WELCOME_TITLE')}
      </AppText>
      <AppText variant="heading" style={[styles.welcome, styles.name]}>
        @{name}
      </AppText>
      <AppText variant="heading" style={styles.welcome}>
        {t('FTUE_START_HERE')}
      </AppText>
    </View>
  );
}

function Caption({ place }: { place: Place }) {
  return (
    <View style={styles.caption}>
      <AppText variant="title" numberOfLines={1} style={styles.placeTitle}>
        {place.title}
      </AppText>
      <AppText variant="body" style={styles.by}>
        {t('COMMON_BY')}{' '}
        <AppText variant="body" style={styles.creator}>
          {place.creator}
        </AppText>
      </AppText>
    </View>
  );
}

/**
 * First-run welcome: the `ftue` places as a snap carousel (a single place is a static card), JUMP IN
 * for the centred one and a Skip link to Discover. Without `places` it fills when the list lands.
 */
export function FtueScreen({
  username,
  places,
  initialIndex = 0,
  skipLabel = 'FTUE_SKIP',
  onJump,
  onSkip,
}: {
  username: string;
  places?: Place[];
  initialIndex?: number;
  skipLabel?: 'FTUE_SKIP' | 'FTUE_SKIP_AND_EXPLORE';
  onJump: (place: Place) => void;
  onSkip: () => void;
}) {
  const api = useApi();
  const insets = useSafeAreaInsets();
  const [loaded, setLoaded] = useState<Place[]>([]);
  const [index, setIndex] = useState(initialIndex);
  useEffect(() => {
    if (!places) {
      ftuePlaces(api).then(setLoaded);
    }
  }, [api, places]);

  const list = places ?? loaded;
  const current = list[Math.min(index, list.length - 1)];
  const single = list.length === 1;

  return (
    <View
      style={[
        styles.root,
        { paddingTop: insets.top, paddingBottom: insets.bottom },
      ]}
    >
      <DiscoverBackground seasonal={false} />
      <Headline name={username} />
      {current ? (
        <>
          {single ? (
            <View style={styles.staticCard}>
              <Thumb
                uri={current.image}
                width={u(STATIC_W)}
                height={u(220)}
                radius={6}
              />
            </View>
          ) : (
            <View style={styles.carousel}>
              <SnapCarousel
                data={list}
                itemWidth={CARD_W}
                gap={15}
                inset={30}
                keyOf={place => place.id}
                onIndexChange={setIndex}
                renderItem={place => (
                  <Thumb
                    uri={place.image}
                    width={u(CARD_W)}
                    height={u(CARD_H)}
                    radius={6}
                  />
                )}
              />
            </View>
          )}
          <Caption place={current} />
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('COMMON_JUMP_IN')}
            onPress={() => onJump(current)}
            style={({ pressed }) => [styles.jump, pressed && styles.pressed]}
          >
            <AppText variant="button" style={styles.jumpText}>
              {t('COMMON_JUMP_IN')}
            </AppText>
            <JumpGlyph />
          </Pressable>
        </>
      ) : null}
      <Pressable
        accessibilityRole="button"
        hitSlop={12}
        onPress={onSkip}
        style={styles.skip}
      >
        <AppText variant="body" style={styles.skipText}>
          {t(skipLabel)}
        </AppText>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    ...StyleSheet.absoluteFill,
    backgroundColor: colors.purple500,
    alignItems: 'center',
    justifyContent: 'flex-start',
  },
  headline: { alignItems: 'center', marginTop: u(60), marginBottom: u(38) },
  welcome: { fontSize: u(24), lineHeight: u(29), textAlign: 'center' },
  name: { color: colors.lavender },
  carousel: { alignSelf: 'stretch' },
  staticCard: { borderRadius: u(6) },
  caption: { alignItems: 'center', marginTop: u(11) },
  placeTitle: { fontSize: u(22) },
  by: { marginTop: u(4), fontSize: u(13), color: colors.white70 },
  creator: {
    fontSize: u(13),
    fontFamily: 'Inter-SemiBold',
    color: colors.snow,
  },
  jump: {
    marginTop: u(42),
    width: u(275),
    height: u(47),
    borderRadius: u(6),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(8),
    backgroundColor: colors.ruby,
  },
  jumpText: { fontFamily: 'Inter-Bold', fontSize: u(15) },
  skip: { marginTop: u(21) },
  skipText: {
    fontSize: u(15),
    fontFamily: 'Inter-SemiBold',
    color: colors.lavender,
    textDecorationLine: 'underline',
  },
  pressed: { opacity: 0.85 },
});
