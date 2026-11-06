import { Pressable, StyleSheet, useWindowDimensions, View } from 'react-native';
import { AppText } from './AppText';
import { Button } from './Button';
import { Icon } from './Icon';
import { SearchField } from './SearchField';
import { colors, fonts, radii } from './tokens';
import { u } from './scale';
import { t } from './i18n';

/** Figma `Header`. Portrait: Title / TitleSearch / Back / BackSearch / SearchOpen, with the
 * optional `Badge/Credits` (and the Ticket action next to it on Back). Landscape: Title /
 * TitleButton, where `action` is a Secondary outline button. Orientation follows the window. */
export function Header({
  title,
  onBack,
  onSearch,
  search,
  credits,
  onTicket,
  action,
}: {
  title: string;
  onBack?: () => void;
  /** Shows the Search icon; the screen switches to `search` (SearchOpen) itself. */
  onSearch?: () => void;
  /** SearchOpen: back + the search field in place of the title. */
  search?: {
    value: string;
    onChangeText: (text: string) => void;
    onSubmit?: (text: string) => void;
    onClose: () => void;
    placeholder?: string;
  };
  credits?: { amount: number; onPress?: () => void };
  onTicket?: () => void;
  action?: { label: string; onPress: () => void };
}) {
  const { width, height } = useWindowDimensions();
  if (width > height) {
    return (
      <View style={styles.landscape}>
        <AppText style={styles.titleLandscape}>{title}</AppText>
        {action && (
          <Button tone="outline" label={action.label} onPress={action.onPress} />
        )}
      </View>
    );
  }
  if (search) {
    return (
      <View style={[styles.portrait, styles.searchOpen]}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('APP_BACK')}
          hitSlop={u(8)}
          onPress={search.onClose}
        >
          <Icon name="back" size={30} />
        </Pressable>
        <View style={styles.grow}>
          <SearchField
            autoFocus
            value={search.value}
            placeholder={search.placeholder}
            onChangeText={search.onChangeText}
            onSubmit={search.onSubmit}
          />
        </View>
      </View>
    );
  }
  return (
    <View style={[styles.portrait, onBack && styles.portraitBack]}>
      <View style={styles.titleGroup}>
        {onBack && (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_BACK')}
            hitSlop={u(8)}
            onPress={onBack}
          >
            <Icon name="back" size={30} />
          </Pressable>
        )}
        <AppText numberOfLines={1} style={styles.title}>
          {title}
        </AppText>
      </View>
      <View style={styles.actions}>
        {onSearch && (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_SEARCH')}
            hitSlop={u(8)}
            onPress={onSearch}
          >
            <Icon name="search" size={30} />
          </Pressable>
        )}
        {onBack && onTicket && (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_TICKET')}
            hitSlop={u(8)}
            onPress={onTicket}
          >
            <Icon name="ticket" size={27} />
          </Pressable>
        )}
        {credits && (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_CREDITS')}
            disabled={!credits.onPress}
            onPress={credits.onPress}
            style={styles.credits}
          >
            <Icon name="credits" size={20} />
            <AppText style={styles.creditsAmount}>{String(credits.amount)}</AppText>
          </Pressable>
        )}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  portrait: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    justifyContent: 'space-between',
    paddingTop: u(20),
    paddingBottom: u(12),
    paddingHorizontal: u(24),
  },
  portraitBack: { paddingBottom: u(15) },
  searchOpen: { gap: u(12), paddingBottom: u(10) },
  grow: { flex: 1 },
  titleGroup: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(4),
    flexShrink: 1,
  },
  title: {
    fontFamily: fonts.bold,
    fontSize: u(24),
    color: colors.snow,
    flexShrink: 1,
  },
  actions: { flexDirection: 'row', alignItems: 'center', gap: u(16) },
  credits: {
    height: u(30),
    paddingHorizontal: u(9),
    gap: u(5),
    flexDirection: 'row',
    alignItems: 'center',
    borderRadius: radii.pill,
    backgroundColor: colors.black40,
  },
  creditsAmount: { fontFamily: fonts.medium, fontSize: u(16), color: colors.snow },
  landscape: {
    minHeight: u(37),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: u(8),
  },
  titleLandscape: {
    fontFamily: fonts.bold,
    fontSize: u(22),
    color: colors.snow,
    flexShrink: 1,
  },
});
