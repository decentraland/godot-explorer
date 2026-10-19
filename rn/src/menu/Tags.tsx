import { StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, LinearFill, u } from '../ds';
import { formatUsers } from '../discover/format';
import { t } from '../i18n';

const MINT = '#34CE76';
const GREEN_DOT = '#25D321';

function Tag({
  label,
  color = colors.snow,
  fill,
}: {
  label: string;
  color?: string;
  fill: string;
}) {
  return (
    <View style={[styles.tag, { backgroundColor: fill }]}>
      <AppText variant="caption" style={[styles.tagText, { color }]}>
        {label}
      </AppText>
    </View>
  );
}

export const LiveTag = () => <Tag label={t('MENU_LIVE')} fill={colors.ruby} />;

export const TrendingTag = () => <Tag label={t('MENU_TRENDING')} fill={MINT} />;

/** Upcoming events: white fill, ruby text and border. */
export function SoonTag({ label }: { label: string }) {
  return (
    <View style={[styles.tag, styles.soon]}>
      <AppText variant="caption" style={[styles.tagText, styles.soonText]}>
        {label}
      </AppText>
    </View>
  );
}

export function UsersTag({ count }: { count: number }) {
  return (
    <View style={[styles.tag, styles.users]}>
      <View style={styles.dot} />
      <Icon name="user" size={11} color={colors.snow} strokeWidth={2} />
      <AppText variant="caption" style={[styles.tagText, styles.usersText]}>
        {formatUsers(count)}
      </AppText>
    </View>
  );
}

export function FeaturedTag() {
  return (
    <View style={[styles.tag, styles.featured]}>
      <LinearFill
        angle={90}
        stops={[
          { offset: 0, color: colors.orange },
          { offset: 1, color: '#D63FA8' },
        ]}
      />
      <AppText variant="caption" style={[styles.tagText, styles.featuredText]}>
        {t('MENU_FEATURED')}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  tag: {
    height: u(16),
    paddingHorizontal: u(4),
    borderRadius: u(3),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(2),
    overflow: 'hidden',
  },
  tagText: { fontSize: u(10), lineHeight: u(13), color: colors.snow },
  soon: {
    backgroundColor: colors.snow,
    borderWidth: 0.5,
    borderColor: colors.ruby,
  },
  soonText: { color: colors.ruby },
  users: { backgroundColor: '#111111' },
  usersText: { fontSize: u(11) },
  dot: {
    width: u(4),
    height: u(4),
    borderRadius: u(2),
    backgroundColor: GREEN_DOT,
  },
  featured: { backgroundColor: colors.orange },
  featuredText: { fontFamily: 'Inter-Medium' },
});
