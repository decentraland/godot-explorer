import { Fragment, ReactNode } from 'react';
import { Linking, Pressable, StyleSheet, Text, View } from 'react-native';
import { SvgXml } from 'react-native-svg';
import { AppText, colors, Icon, IconName } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { JumpGlyph } from './JumpGlyph';
import { splitLinks } from './links';
import type { StatCell } from './stats';
import { Thumb } from './Thumb';

const HERO_RATIO = 368 / 624;
const HEART = (fill: string, stroke: string) =>
  `<svg viewBox="0 0 24 24"><path d="M12 20.5 C5 15 3 11.5 3 8.5 A4.5 4.5 0 0 1 12 6.5 A4.5 4.5 0 0 1 21 8.5 C21 11.5 19 15 12 20.5 Z" fill="${fill}" stroke="${stroke}" stroke-width="2" stroke-linejoin="round"/></svg>`;

export function HeartButton({
  on,
  onPress,
}: {
  on: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('PLACE_FAVORITE')}
      accessibilityState={{ selected: on }}
      hitSlop={10}
      onPress={onPress}
    >
      <SvgXml
        xml={on ? HEART(colors.ruby, colors.ruby) : HEART('none', colors.snow)}
        width={u(28)}
        height={u(28)}
      />
    </Pressable>
  );
}

/** 47dp violet square; `on` fills the glyph (liked / disliked). */
export function SquareButton({
  icon,
  label,
  on = false,
  onPress,
}: {
  icon: IconName;
  label: string;
  on?: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected: on }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.square,
        on && styles.squareOn,
        pressed && styles.pressed,
      ]}
    >
      <Icon
        name={icon}
        size={icon === 'calendar' ? 28 : 22}
        strokeWidth={2.2}
        color={on ? colors.lavender : colors.snow}
        fill={on ? colors.lavender : undefined}
      />
    </Pressable>
  );
}

export function JumpButton({
  onPress,
  glyph = true,
}: {
  onPress: () => void;
  glyph?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('COMMON_JUMP_IN')}
      onPress={onPress}
      style={({ pressed }) => [styles.jump, pressed && styles.pressed]}
    >
      <AppText variant="button" style={styles.jumpText}>
        {t('COMMON_JUMP_IN')}
      </AppText>
      {glyph ? <JumpGlyph /> : null}
    </Pressable>
  );
}

export function ActionRow({ children }: { children: ReactNode }) {
  return <View style={styles.actions}>{children}</View>;
}

/** Hero image with an optional overlay (event badges) and the download-size strip. */
export function Hero({
  image,
  downloadMb,
  children,
}: {
  image: string;
  downloadMb?: number | null;
  children?: ReactNode;
}) {
  return (
    <View style={styles.hero}>
      <Thumb uri={image} width="100%" height="100%" radius={6} />
      {children}
      {downloadMb ? (
        <View style={styles.strip}>
          <Icon name="warning" size={13} strokeWidth={2} />
          <AppText variant="caption" style={styles.stripText}>
            {t('PLACE_DOWNLOAD_WARNING', { megabytes: downloadMb })}
          </AppText>
        </View>
      ) : null}
    </View>
  );
}

/** Stat cells spread edge to edge with dividers between three, or at the two ends when there are fewer. */
export function StatRow({
  cells,
  layout,
}: {
  cells: StatCell[];
  layout: 'thirds' | 'ends';
}) {
  if (cells.length === 0) {
    return null;
  }
  return (
    <View style={styles.stats}>
      {cells.map((cell, i) => (
        <Fragment key={cell.kind}>
          {layout === 'thirds' && i > 0 ? (
            <View style={styles.statRule} />
          ) : null}
          <View style={styles.stat}>
            <Icon
              name={cell.icon}
              size={15}
              color={colors.gray}
              strokeWidth={2}
            />
            <AppText variant="body" numberOfLines={1} style={styles.statText}>
              {cell.text}
            </AppText>
          </View>
        </Fragment>
      ))}
    </View>
  );
}

export function Description({ text }: { text: string }) {
  return (
    <View>
      <AppText variant="label" style={styles.descLabel}>
        {t('COMMON_DESCRIPTION')}
      </AppText>
      <Text allowFontScaling={false} style={styles.desc}>
        {splitLinks(text).map((part, i) =>
          part.url ? (
            <Text
              key={i}
              style={styles.link}
              onPress={() => Linking.openURL(part.url!).catch(() => undefined)}
            >
              {part.text}
            </Text>
          ) : (
            part.text
          ),
        )}
      </Text>
    </View>
  );
}

/** Small tinted category tag under the description. */
export function CategoryPill({
  label,
  icon,
}: {
  label: string;
  icon: IconName;
}) {
  return (
    <View style={styles.pill}>
      <Icon name={icon} size={9} strokeWidth={2} />
      <Text style={styles.pillText}>{label}</Text>
    </View>
  );
}

export const Divider = () => <View style={styles.rule} />;

export function Byline({ creator }: { creator: string }) {
  return creator ? (
    <AppText variant="body" style={styles.by}>
      {t('COMMON_BY')}{' '}
      <AppText variant="body" style={styles.creator}>
        {creator}
      </AppText>
    </AppText>
  ) : null;
}

export const SKELETON_HERO_RATIO = HERO_RATIO;

const styles = StyleSheet.create({
  square: {
    width: u(47),
    height: u(47),
    borderRadius: u(6),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.violetSquare,
  },
  squareOn: { backgroundColor: '#5A188F' },
  jump: {
    flex: 1,
    height: u(47),
    borderRadius: u(6),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(8),
    backgroundColor: colors.ruby,
  },
  jumpText: { fontFamily: 'Inter-Bold', fontSize: u(15) },
  actions: { flexDirection: 'row', gap: u(6), marginTop: u(14) },
  hero: {
    width: '100%',
    aspectRatio: 1 / HERO_RATIO,
    overflow: 'hidden',
    borderRadius: u(6),
    marginTop: u(16),
  },
  strip: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: u(24),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(6),
    backgroundColor: 'rgba(22,21,24,0.85)',
  },
  stripText: { fontSize: u(12), color: colors.snow },
  stats: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginTop: u(13),
    paddingHorizontal: u(6),
  },
  statRule: { width: 1, height: u(20), backgroundColor: colors.white30 },
  stat: { flexDirection: 'row', alignItems: 'center', gap: u(6) },
  statText: { fontSize: u(15), color: colors.pearl },
  descLabel: { fontSize: u(12), color: '#8E60AA', letterSpacing: 0.5 },
  desc: {
    marginTop: u(7),
    fontSize: u(12.8),
    lineHeight: u(17),
    color: colors.pearl,
    fontFamily: 'Inter-Regular',
  },
  pill: {
    height: u(22),
    paddingHorizontal: u(8),
    borderRadius: u(11),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(4),
    backgroundColor: '#4A2165',
  },
  pillText: {
    fontSize: u(9.5),
    fontFamily: 'Inter-Bold',
    color: colors.snow,
  },
  link: { color: colors.periwinkle },
  rule: {
    height: 1,
    backgroundColor: '#7C599A',
    marginTop: u(26),
    marginBottom: u(17),
  },
  by: { marginTop: u(6), fontSize: u(13), color: colors.lavender },
  creator: {
    fontSize: u(13),
    color: '#C4B5CD',
    fontFamily: 'Inter-SemiBold',
  },
  pressed: { opacity: 0.85 },
});
