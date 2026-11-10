// Design System Mobile: the Cards_Backpack, Cards_Discover and Skeleton pages and the Modals page
// `Sheet`, every variant.
import { ScrollView, StyleSheet, View } from 'react-native';
import {
  AppText,
  AvatarCard,
  BrandBackground,
  Button,
  colors,
  DetailRow,
  DiscoverCard,
  DiscoverCardSkeleton,
  EventBadge,
  EventTimeTag,
  fonts,
  ItemCard,
  LinearFill,
  ProfileCard,
  RarityBackground,
  Rarity,
  Sheet,
  Skeleton,
  u,
} from '@dcl/ui-native';
import { registerPreview } from './registry';

const noop = () => {};
const RARITIES: Rarity[] = [
  'base',
  'common',
  'uncommon',
  'rare',
  'epic',
  'legendary',
  'exotic',
  'mythic',
  'unique',
];
const art = (
  <LinearFill
    angle={135}
    stops={[
      { offset: 0, color: '#2A6FB5' },
      { offset: 1, color: '#C7742E' },
    ]}
  />
);

function Label({ children }: { children: string }) {
  return <AppText style={styles.label}>{children}</AppText>;
}

registerPreview('extra/ds-cards-backpack', {
  title: 'DS: Cards_Backpack',
  orientation: 'portrait',
  render: () => (
    <ScrollView style={styles.page} contentContainerStyle={styles.content}>
      <Label>Rarity-Backgrounds</Label>
      <View style={styles.wrap}>
        {RARITIES.map(rarity => (
          <View key={rarity} style={styles.swatch}>
            <RarityBackground rarity={rarity} />
          </View>
        ))}
        <View style={styles.swatch}>
          <RarityBackground disabled />
        </View>
      </View>
      <Label>Cards-Wearables</Label>
      <View style={styles.wrap}>
        <ItemCard style={styles.card} onPress={noop} />
        <ItemCard style={styles.card} state="unequip" actionLabel="Unequip" />
        <ItemCard style={styles.card} state="disabled" />
        <ItemCard style={styles.card} state="equipped" />
        <ItemCard style={styles.card} state="equip" actionLabel="Equip" />
        <ItemCard style={styles.card} state="detail" actionLabel="Detail" />
        <ItemCard style={styles.card} rarity="epic" price="0" />
        <ItemCard style={styles.card} rarity="mythic" newLabel="New" />
        <ItemCard style={styles.card} state="selected" />
        <ItemCard style={styles.card} empty />
        <ItemCard style={styles.card} empty state="selected" />
        <ItemCard style={styles.card} empty state="disabled" />
      </View>
      <Label>Card-Profile</Label>
      <View style={styles.row}>
        <ProfileCard
          title="Item Name Item Name Item Name"
          rarity="epic"
          price="0"
          art={art}
        />
        <ProfileCard
          type="emote"
          title="Item Name Item Name Item Name"
          rarity="epic"
          price="0"
          art={art}
        />
        <ProfileCard
          type="name"
          title="NAME"
          name="Name"
          nameSuffix="DCL.ETH"
          price="0"
        />
      </View>
      <Label>Avatar-Card</Label>
      <View style={styles.row}>
        <AvatarCard selected />
        <AvatarCard />
      </View>
    </ScrollView>
  ),
});

registerPreview('extra/ds-cards-discover', {
  title: 'DS: Cards_Discover',
  orientation: 'portrait',
  render: () => (
    <ScrollView style={styles.page} contentContainerStyle={styles.content}>
      <View style={styles.row}>
        <DiscoverCard
          type="place"
          title="Experience Name two lines example elipsis"
          art={art}
          users="000"
          likeRate="100%"
        />
        <DiscoverCard
          type="place"
          title="Experience Name two lines example elipsis"
          image=""
          users="000"
          likeRate="100%"
        />
      </View>
      {[art, undefined].map((fill, i) => (
        <DiscoverCard
          key={i}
          type="event"
          title="Experience Name two lines example elipsis"
          art={fill}
          image=""
          byLabel="by"
          creator="Name Creator"
          tags={
            <>
              <EventTimeTag kind="live" label="LIVE" players={0} />
              <EventBadge type="trending" label="Trending" />
            </>
          }
        />
      ))}
      <View style={styles.wrap}>
        <DetailRow type="players" value="000" />
        <DetailRow type="likes" value="100%" />
        <DetailRow type="coords" value="-000,-000" />
        <DetailRow type="date" value="Weekly" />
        <DetailRow type="duration" value="1 Hr" />
        <DetailRow type="world" value="Aaaaaaaaaaaaaaa" />
      </View>
      <View style={styles.row}>
        <DiscoverCardSkeleton type="place" />
        <DiscoverCardSkeleton type="event" />
      </View>
    </ScrollView>
  ),
});

registerPreview('extra/ds-skeleton', {
  title: 'DS: Skeleton and Background',
  orientation: 'portrait',
  render: () => (
    <View style={styles.page}>
      <BrandBackground glow />
      <View style={styles.content}>
        <Skeleton width={u(100)} height={20} />
        <Skeleton width={u(100)} height={20} animation="pulse" />
        <View style={styles.light}>
          <Skeleton width={u(100)} height={20} theme="light" />
          <Skeleton
            width={u(100)}
            height={20}
            theme="light"
            animation="pulse"
          />
        </View>
      </View>
    </View>
  ),
});

registerPreview('extra/ds-sheet', {
  title: 'DS: Sheet',
  orientation: 'portrait',
  render: () => (
    <View style={styles.sheetPage}>
      <Sheet height="collapsed" />
      <Sheet>
        <Label>Handle=On, Height=Hug</Label>
      </Sheet>
      <Sheet
        handle={false}
        footer={
          <View style={styles.footerRow}>
            <Button label="Edit" tone="secondary" big onPress={noop} />
            <View style={styles.grow}>
              <Button label="Next" big fullWidth onPress={noop} />
            </View>
          </View>
        }
      >
        <View style={styles.entry}>
          <AppText style={styles.entryTitle}>Choose your look</AppText>
          <View style={styles.avatars}>
            <AvatarCard selected />
            <AvatarCard />
            <AvatarCard />
            <AvatarCard />
          </View>
        </View>
      </Sheet>
    </View>
  ),
});

const styles = StyleSheet.create({
  page: { flex: 1, backgroundColor: colors.paleBlack },
  content: { padding: u(16), gap: u(14) },
  label: { color: colors.snow },
  wrap: { flexDirection: 'row', flexWrap: 'wrap', gap: u(10) },
  row: { flexDirection: 'row', gap: u(16) },
  swatch: {
    width: u(30),
    height: u(30),
    borderRadius: u(6),
    overflow: 'hidden',
  },
  card: { width: u(96) },
  light: { gap: u(14), padding: u(10), backgroundColor: colors.snow },
  sheetPage: {
    flex: 1,
    justifyContent: 'flex-end',
    gap: u(16),
    backgroundColor: colors.paleBlack,
  },
  footerRow: { flexDirection: 'row', gap: u(9) },
  grow: { flex: 1 },
  entry: { gap: u(20) },
  entryTitle: {
    fontFamily: fonts.bold,
    fontSize: u(18),
    color: colors.snow,
    textAlign: 'center',
  },
  avatars: { flexDirection: 'row', gap: u(12), overflow: 'hidden' },
});
