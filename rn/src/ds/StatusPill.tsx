import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors } from './tokens';
import { u } from './scale';

export type StatusKind = 'live' | 'trending' | 'soon' | 'count';

const kinds: Record<StatusKind, { bg: string; fg: string; icon?: IconName }> = {
  live: { bg: colors.ruby, fg: colors.snow },
  trending: { bg: colors.black40, fg: colors.snow, icon: 'flame' },
  soon: { bg: colors.orange, fg: colors.shadow },
  count: { bg: colors.black40, fg: colors.snow, icon: 'people' },
};

/** Small badge on cards: LIVE, IN 5 MINS, Trending, attendee count. */
export function StatusPill({
  kind,
  label,
}: {
  kind: StatusKind;
  label: string;
}) {
  const { bg, fg, icon } = kinds[kind];
  return (
    <View style={[styles.pill, { backgroundColor: bg }]}>
      {icon && <Icon name={icon} size={12} color={fg} strokeWidth={2} />}
      <AppText variant="caption" style={[styles.text, { color: fg }]}>
        {label}
      </AppText>
    </View>
  );
}

/** Link chip: icon + periwinkle title on a dark pill, with an optional remove X. */
export function Chip({
  label,
  icon,
  onPress,
  onRemove,
}: {
  label: string;
  icon?: IconName;
  onPress?: () => void;
  onRemove?: () => void;
}) {
  return (
    <View style={styles.chip}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={label}
        disabled={!onPress}
        onPress={onPress}
        style={styles.chipMain}
      >
        {icon && <Icon name={icon} size={16} color={colors.periwinkle} />}
        <AppText variant="caption" style={styles.chipText}>
          {label}
        </AppText>
      </Pressable>
      {onRemove && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Remove ${label}`}
          hitSlop={8}
          onPress={onRemove}
        >
          <Icon name="close" size={14} color={colors.periwinkle} />
        </Pressable>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  pill: {
    height: u(18),
    paddingHorizontal: u(6),
    borderRadius: u(9),
    flexDirection: 'row',
    alignItems: 'center',
    alignSelf: 'flex-start',
    gap: u(3),
  },
  text: { fontSize: u(10), textTransform: 'uppercase' },
  chip: {
    height: u(32),
    paddingHorizontal: u(12),
    borderRadius: u(8),
    flexDirection: 'row',
    alignItems: 'center',
    alignSelf: 'flex-start',
    backgroundColor: colors.black40,
    gap: u(8),
  },
  chipMain: { flexDirection: 'row', alignItems: 'center', gap: u(6) },
  chipText: { fontSize: u(14), color: colors.periwinkle },
});
