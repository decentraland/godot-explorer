import { Share, StyleSheet, View } from 'react-native';
import { AppText, Skeleton } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { Place } from './api';
import { placeJumpLink } from './eventTime';
import { SheetShell, SheetSnap } from './SheetShell';
import {
  ActionRow,
  Byline,
  CategoryPill,
  Description,
  Divider,
  HeartButton,
  Hero,
  JumpButton,
  SquareButton,
  StatRow,
} from './SheetParts';
import { placeStats } from './stats';
import type { PlaceDetail, VoteState } from './types';

const COLLAPSED = 0.575;
const CATEGORY_ICON = {
  art: 'pencil',
  business: 'flag',
  social: 'people',
} as const;

export const sharePlace = (place: Place) =>
  Share.share({
    message: t('SHARE_PLACE_MESSAGE', {
      place: place.title,
      link: placeJumpLink(place),
    }),
  }).catch(() => undefined);

function Skeletons() {
  return (
    <View>
      <Skeleton width="100%" height={17} radius={2} style={styles.gap} />
      <Skeleton width="100%" height={17} radius={2} style={styles.gap} />
      <Skeleton width="37%" height={15} radius={2} style={styles.gap} />
      <View style={styles.hero}>
        <Skeleton width="100%" height={183} radius={6} />
      </View>
      <View style={styles.statSkeleton}>
        {[0, 1, 2].map(i => (
          <Skeleton key={i} width={u(60)} height={20} radius={2} />
        ))}
      </View>
    </View>
  );
}

/** Place / world sheet. `detail` null renders the loading skeleton (buttons stay live). */
export function PlaceSheet({
  detail,
  initial,
  onClose,
  onJumpIn,
  onFavorite,
  onVote,
}: {
  detail: PlaceDetail | null;
  initial?: SheetSnap;
  onClose: () => void;
  onJumpIn: () => void;
  onFavorite: (on: boolean) => void;
  onVote: (vote: VoteState) => void;
}) {
  const place = detail?.place;
  const stats = place ? placeStats(place) : null;
  const toggle = (vote: 'like' | 'dislike') =>
    onVote(detail?.vote === vote ? null : vote);

  return (
    <SheetShell
      fraction={COLLAPSED}
      initial={initial}
      onClose={onClose}
      footer={
        <ActionRow>
          <SquareButton
            icon="share"
            label={t('COMMON_SHARE')}
            onPress={() => place && sharePlace(place)}
          />
          <SquareButton
            icon="like"
            label={t('PLACE_LIKE')}
            on={detail?.vote === 'like'}
            onPress={() => toggle('like')}
          />
          <SquareButton
            icon="dislike"
            label={t('PLACE_DISLIKE')}
            on={detail?.vote === 'dislike'}
            onPress={() => toggle('dislike')}
          />
          <JumpButton onPress={onJumpIn} />
        </ActionRow>
      }
    >
      {snap =>
        place && stats && detail ? (
          <View>
            <View style={styles.head}>
              <View style={styles.headText}>
                <AppText variant="title" numberOfLines={2} style={styles.title}>
                  {place.title}
                </AppText>
                <Byline creator={place.creator} />
              </View>
              <HeartButton
                on={detail.favorite}
                onPress={() => onFavorite(!detail.favorite)}
              />
            </View>
            <Hero image={place.image} downloadMb={detail.downloadMb} />
            <StatRow cells={stats.cells} layout={stats.layout} />
            {snap === 'expanded' ? (
              <View>
                <Divider />
                {place.description ? (
                  <Description text={place.description} />
                ) : null}
                {place.categories?.length ? (
                  <>
                    <Divider />
                    <View style={styles.chips}>
                      {place.categories.map(category => (
                        <CategoryPill
                          key={category}
                          label={category.toUpperCase()}
                          icon={
                            CATEGORY_ICON[
                              category as keyof typeof CATEGORY_ICON
                            ] ?? 'flag'
                          }
                        />
                      ))}
                    </View>
                  </>
                ) : null}
              </View>
            ) : null}
          </View>
        ) : (
          <Skeletons />
        )
      }
    </SheetShell>
  );
}

const styles = StyleSheet.create({
  head: { flexDirection: 'row', alignItems: 'flex-start', marginTop: u(6) },
  headText: { flex: 1, paddingRight: u(12) },
  title: {
    fontSize: u(20.5),
    lineHeight: u(25.5),
    fontFamily: 'Inter-SemiBold',
  },
  chips: { flexDirection: 'row', flexWrap: 'wrap', gap: u(5) },
  gap: { marginTop: u(10) },
  hero: { marginTop: u(16) },
  statSkeleton: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginTop: u(16),
  },
});
