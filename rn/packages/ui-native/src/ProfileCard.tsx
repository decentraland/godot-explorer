import { ReactNode } from 'react';
import {
  Pressable,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import { AppText } from './AppText';
import { RadialFill } from './Gradient';
import { Icon } from './Icon';
import { ItemCard } from './ItemCard';
import { colors, fonts, Rarity, radius } from './tokens';
import { u } from './scale';

export type ProfileCardType = 'wearable' | 'emote' | 'name';

const glow = (color: string, opacities: number[]) =>
  opacities.map((opacity, i) => ({
    offset: i === opacities.length - 1 ? 1 : i * 0.15,
    color,
    opacity,
  }));
const VIOLET = glow(
  colors.nameCardViolet,
  [0.938, 0.89, 0.733, 0.478, 0.227, 0.073, 0.015, 0],
);
const PEACH = glow(
  colors.nameCardPeach,
  [0.914, 0.861, 0.693, 0.443, 0.207, 0.067, 0.014, 0],
);

/** Figma `Card-Profile` 825:7523 (75 dp wide): a wearable or emote `Cards-Wearables` tile, or the
 * NAME tile, over an info plate with the title and the credits price. */
export function ProfileCard({
  type = 'wearable',
  title,
  thumbnail,
  rarity,
  price,
  name,
  nameSuffix,
  art,
  onPress,
  style,
}: {
  type?: ProfileCardType;
  title: string;
  thumbnail?: string;
  rarity?: Rarity;
  price?: string;
  /** Type=Name: the name drawn on the tile and the suffix under it (`DCL.ETH`). */
  name?: string;
  nameSuffix?: string;
  art?: ReactNode;
  onPress?: () => void;
  style?: StyleProp<ViewStyle>;
}) {
  const isName = type === 'name';
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={title}
      disabled={!onPress}
      onPress={onPress}
      style={[styles.card, style]}
    >
      {isName ? (
        <View style={styles.nameTile}>
          <View style={styles.nameClip}>
            <RadialFill
              stops={VIOLET}
              cx={0.271}
              cy={0.272}
              rx={1.301}
              ry={1.308}
            />
            <RadialFill
              stops={PEACH}
              cx={0.852}
              cy={1.173}
              rx={1.37}
              ry={1.163}
            />
          </View>
          <Icon name="shopNames" size={14} color={colors.snow} />
          {name ? (
            <AppText numberOfLines={1} style={styles.nameText}>
              {name}
            </AppText>
          ) : null}
          {nameSuffix ? (
            <AppText numberOfLines={1} style={styles.nameSuffix}>
              {nameSuffix}
            </AppText>
          ) : null}
        </View>
      ) : (
        <ItemCard thumbnail={thumbnail} rarity={rarity} art={art} />
      )}
      <View style={[styles.info, isName && styles.infoName]}>
        <AppText numberOfLines={2} style={styles.title}>
          {title}
        </AppText>
        {price !== undefined && (
          <View style={styles.price}>
            <Icon name="credits" size={10} />
            <AppText style={styles.priceText}>{price}</AppText>
          </View>
        )}
      </View>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  card: {
    width: u(75),
    borderRadius: radius.sm,
    backgroundColor: colors.black40,
  },
  nameTile: {
    aspectRatio: 1,
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(2),
    borderRadius: radius.md,
    borderWidth: u(0.78),
    borderColor: colors.snow,
    backgroundColor: colors.nameCard,
  },
  nameClip: {
    ...StyleSheet.absoluteFill,
    borderRadius: radius.md,
    overflow: 'hidden',
  },
  nameText: { fontFamily: fonts.bold, fontSize: u(12), color: colors.snow },
  nameSuffix: { fontSize: u(9), color: colors.snow },
  info: { padding: u(5), gap: u(3) },
  infoName: { height: u(46), justifyContent: 'space-between' },
  title: { fontFamily: fonts.semibold, fontSize: u(8), color: colors.snow },
  price: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(2.5),
    paddingVertical: u(2),
  },
  priceText: { fontFamily: fonts.medium, fontSize: u(8), color: colors.snow },
});
