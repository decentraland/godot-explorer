import { ReactNode, useEffect, useMemo, useState } from 'react';
import { Keyboard, StyleSheet, View } from 'react-native';
import { colors } from '../ds';
import { t } from '../i18n';
import { Place } from './api';
import { CategoryScreen } from './CategoryScreen';
import { DiscoverBackground } from './DiscoverBackground';
import { DiscoverHeader } from './DiscoverHeader';
import { DiscoverHome } from './DiscoverHome';
import { usePlayed } from './history';
import { MessageState } from './MessageState';
import { SearchView } from './SearchView';
import { SeasonalOverlay } from './SeasonalOverlay';
import type { SearchState } from './search';
import type { Category } from './types';
import { useOpeners } from './useOpeners';
import './routes';

function useKeyboardVisible(): boolean {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const show = Keyboard.addListener('keyboardDidShow', () =>
      setVisible(true),
    );
    const hide = Keyboard.addListener('keyboardDidHide', () =>
      setVisible(false),
    );
    return () => {
      show.remove();
      hide.remove();
    };
  }, []);
  return visible;
}

/**
 * Discover tab: banners and rails, search mode and the category page. `onBack` makes it the pushed
 * variant (chevron, no nav); `footer` is the bottom nav of the tab shell. The `start*` props open it
 * mid-flow for the gallery.
 */
export function DiscoverScreen({
  footer,
  onJumpIn,
  onBack,
  played,
  now,
  startSearch,
  startCategory,
  startCategorySearch = false,
  bannerIndex,
  seasonal,
  startScrolled,
}: {
  footer?: ReactNode;
  onJumpIn: (place: Place) => void;
  onBack?: () => void;
  played?: Place[];
  now?: number;
  startSearch?: Partial<SearchState>;
  startCategory?: Category;
  startCategorySearch?: boolean;
  bannerIndex?: number;
  seasonal?: boolean;
  startScrolled?: boolean;
}) {
  const open = useOpeners();
  const history = usePlayed();
  const [searching, setSearching] = useState(startSearch !== undefined);
  const [category, setCategory] = useState<Category | null>(
    startCategory ?? null,
  );
  const [offline, setOffline] = useState<{
    offline: boolean;
    reload: () => void;
  }>({
    offline: false,
    reload: () => undefined,
  });
  const clock = useMemo(() => now ?? Date.now(), [now]);
  const keyboard = useKeyboardVisible();
  const recent = played ?? history;

  const home = (
    <DiscoverHome
      played={recent}
      now={clock}
      bannerIndex={bannerIndex}
      startScrolled={startScrolled}
      onJump={onJumpIn}
      onCategory={setCategory}
      onOffline={(isOffline, reload) =>
        setOffline({ offline: isOffline, reload })
      }
    />
  );

  const body = () => {
    if (category) {
      return (
        <CategoryScreen
          category={category}
          startSearching={startCategorySearch}
          onBack={() => setCategory(null)}
        />
      );
    }
    if (searching) {
      return (
        <SearchView
          initial={startSearch}
          now={clock}
          lastPlayed={recent}
          idleContent={home}
          onBack={() => setSearching(false)}
          onPlace={open.place}
          onEvent={open.event}
        />
      );
    }
    return (
      <>
        <DiscoverHeader
          title={t('DISCOVER_DISCOVER')}
          onBack={onBack}
          searchDisabled={offline.offline}
          onSearch={() => setSearching(true)}
        />
        {offline.offline ? (
          <MessageState
            glyph="offline"
            title={t('DISCOVER_CONNECTION_LOST')}
            body={t('DISCOVER_CONNECTION_LOST_BODY')}
            onRetry={offline.reload}
          />
        ) : null}
        <View style={offline.offline ? styles.hidden : styles.body}>
          {home}
        </View>
      </>
    );
  };

  return (
    <View style={styles.root}>
      <DiscoverBackground seasonal={seasonal} />
      <SeasonalOverlay />
      <View style={styles.body}>{body()}</View>
      {keyboard ? null : footer}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
  body: { flex: 1 },
  hidden: { height: 0, overflow: 'hidden' },
});
