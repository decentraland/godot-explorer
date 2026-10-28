import { ScrollView, StyleSheet, View } from 'react-native';
import { AppText, colors, fonts, Icon, ItemCard, rarityColors } from '../ds';
import type { I18nKey } from '../i18n';
import { t } from '../i18n';
import { u } from '../ds/scale';
import type { EquippedItem } from './types';

const CARD_WIDTH = 75;

/** Horizontal strip of equipped wearables and emotes: art, name and a rarity tag. */
export function EquippedRow({
  items,
  onPress,
}: {
  items: EquippedItem[];
  onPress?: (item: EquippedItem) => void;
}) {
  return (
    <ScrollView
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.row}
    >
      {items.map(item => {
        const tone = rarityColors[item.rarity];
        return (
          <View key={item.id} style={styles.card}>
            <ItemCard
              thumbnail={item.thumbnail}
              rarity={item.rarity}
              label={item.name}
              art={<CategoryArt item={item} />}
              onPress={onPress ? () => onPress(item) : undefined}
            />
            <View style={styles.plate}>
              <AppText variant="caption" numberOfLines={1} style={styles.name}>
                {item.name}
              </AppText>
              <View style={[styles.tag, { backgroundColor: `${tone.dark}40` }]}>
                <AppText
                  variant="caption"
                  style={[styles.tagText, { color: tone.light }]}
                >
                  {t(`PROFILE_RARITY_${item.rarity.toUpperCase()}` as I18nKey)}
                </AppText>
              </View>
            </View>
          </View>
        );
      })}
    </ScrollView>
  );
}

/** Corner category badge, plus an emote silhouette while there is no thumbnail. */
function CategoryArt({ item }: { item: EquippedItem }) {
  return (
    <>
      {item.isEmote && (
        <View style={styles.silhouette}>
          <Icon name="emotes" size={40} color="#0B0412" strokeWidth={2.2} />
        </View>
      )}
      <View style={styles.badge}>
        <Icon
          name={item.isEmote ? 'emotes' : 'wearables'}
          size={9}
          strokeWidth={2}
        />
      </View>
    </>
  );
}

const styles = StyleSheet.create({
  silhouette: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
  },
  badge: {
    position: 'absolute',
    top: u(3),
    left: u(3),
    width: u(14),
    height: u(14),
    borderRadius: u(7),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'rgba(0,0,0,0.45)',
  },
  row: { gap: u(12) },
  card: { width: u(CARD_WIDTH) },
  plate: {
    marginTop: -u(2),
    paddingHorizontal: u(5),
    paddingTop: u(6),
    paddingBottom: u(8),
    borderBottomLeftRadius: u(4),
    borderBottomRightRadius: u(4),
    backgroundColor: '#1B0030',
  },
  name: { color: colors.snow, fontSize: u(9), fontFamily: fonts.medium },
  tag: {
    alignSelf: 'flex-start',
    marginTop: u(5),
    paddingHorizontal: u(4),
    height: u(12),
    justifyContent: 'center',
    borderRadius: u(2),
  },
  tagText: { fontSize: u(7), fontFamily: fonts.bold },
});
