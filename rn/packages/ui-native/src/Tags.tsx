import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { LinearFill } from './Gradient';
import { Icon } from './Icon';
import { colors, fonts, lightRainbow, Rarity, rarityColors } from './tokens';
import { u } from './scale';

export type EventTimeKind =
  | 'live'
  | 'startingSoon'
  | 'inMinutes'
  | 'inHours'
  | 'inDays'
  | 'date';

/** Figma `Tag/EventTime`: Live is Ruby with the player count box, StartingSoon is Snow with Ruby
 * text, the rest are Snow with Shadow text. The app builds the text. */
export function EventTimeTag({
  kind,
  label,
  players,
}: {
  kind: EventTimeKind;
  label: string;
  players?: number;
}) {
  const live = kind === 'live';
  const soon = kind === 'startingSoon';
  return (
    <View style={styles.row}>
      <View
        style={[
          styles.tag,
          live ? styles.tagLive : soon ? styles.tagSoon : styles.tagLater,
        ]}
      >
        <AppText
          style={[
            styles.label,
            live ? styles.textSnow : soon ? styles.textRuby : styles.textShadow,
          ]}
        >
          {label}
        </AppText>
      </View>
      {live && players !== undefined && (
        <View style={styles.players}>
          <View style={styles.dot} />
          <Icon name="players" size={11} />
          <AppText style={styles.count}>{String(players)}</AppText>
        </View>
      )}
    </View>
  );
}

const TRENDING = [
  { offset: 0.019, color: '#22C1C1' },
  { offset: 1, color: '#34C070' },
];

/** Figma `Tag/Event`: a gradient highlight tag, Trending (teal) or Featured (light rainbow). */
export function EventBadge({
  type,
  label,
}: {
  type: 'trending' | 'featured';
  label: string;
}) {
  return (
    <View style={[styles.tag, styles.badge]}>
      {/* Figma CSS angles −73.2° / 147.6°; `LinearFill` is 0 = top→bottom, so angle = 180 − θ. */}
      <LinearFill
        stops={type === 'trending' ? TRENDING : lightRainbow}
        angle={type === 'trending' ? 253 : 32}
      />
      <AppText style={[styles.badgeLabel, styles.textSnow]}>{label}</AppText>
    </View>
  );
}

/** Figma `RarityLabel` 825:6933: the rarity name in its Rarities colour on a 20% tint of it (Base: Gray
 * on Pearl 20%). The caller passes the translated name. */
export function RarityLabel({
  rarity,
  label,
}: {
  rarity: Rarity;
  label: string;
}) {
  const [text, tint] =
    rarity === 'base'
      ? [colors.gray, colors.pearl]
      : [rarityColors[rarity].dark, rarityColors[rarity].dark];
  return (
    <View style={[styles.rarity, { backgroundColor: `${tint}33` }]}>
      <AppText style={[styles.rarityText, { color: text }]}>{label}</AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: u(3) },
  tag: {
    paddingHorizontal: u(4),
    paddingVertical: u(2),
    borderRadius: u(2),
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
  },
  tagLive: { backgroundColor: colors.ruby },
  tagSoon: {
    backgroundColor: colors.snow,
    borderWidth: 1,
    borderColor: colors.ruby,
  },
  tagLater: {
    backgroundColor: colors.snow,
    borderWidth: 1,
    borderColor: colors.shadow,
  },
  label: {
    fontFamily: fonts.semibold,
    fontSize: u(10),
    textTransform: 'uppercase',
  },
  textSnow: { color: colors.snow },
  textRuby: { color: colors.ruby },
  textShadow: { color: colors.shadow },
  players: {
    height: u(15),
    paddingHorizontal: u(4),
    gap: u(1),
    borderRadius: u(2),
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.shadow,
  },
  dot: {
    width: u(5),
    height: u(5),
    borderRadius: u(2.5),
    backgroundColor: colors.green,
  },
  count: { fontFamily: fonts.medium, fontSize: u(10), color: colors.snow },
  badge: { height: u(16) },
  badgeLabel: { fontFamily: fonts.semibold, fontSize: u(10) },
  rarity: {
    alignSelf: 'flex-start',
    paddingHorizontal: u(4),
    paddingVertical: u(2),
    borderRadius: u(3),
  },
  rarityText: {
    fontFamily: fonts.semibold,
    fontSize: u(8),
    lineHeight: u(10),
    textTransform: 'uppercase',
  },
});
