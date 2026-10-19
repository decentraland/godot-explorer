import type { ReactNode } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { colors, Icon } from '../ds';
import { t } from '../i18n';
import { BackpackHeader } from './BackpackHeader';
import { FilterMenu } from './FilterMenu';
import type { EmptyCopy } from './platformPolicy';
import { BackpackController } from './useBackpack';
import { u } from '../ds/scale';

export type SubRow = 'tiles' | 'pills';

export type LayoutProps = {
  subRow?: SubRow;
  controller: BackpackController;
  copy: EmptyCopy;
  /** Portrait only: the app's bottom navigation, or the entry flow's Continue button. */
  footer?: ReactNode;
  onBack?: () => void;
};

export function BackButton({
  onPress,
  style,
}: {
  onPress: () => void;
  style: object;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('BACKPACK_BACK')}
      onPress={onPress}
      hitSlop={12}
      style={[styles.back, style]}
    >
      <Icon name="back" size={22} />
    </Pressable>
  );
}

export function HeaderRow({
  controller,
  filterGap,
}: {
  controller: BackpackController;
  filterGap?: number;
}) {
  const { state, dispatch } = controller;
  return (
    <BackpackHeader
      mode={state.mode}
      filterActive={state.filter !== 'all'}
      menuOpen={state.filterMenuOpen}
      filterGap={filterGap}
      onMode={mode => dispatch({ type: 'mode', mode })}
      onFilter={() => dispatch({ type: 'toggleFilterMenu' })}
    />
  );
}

/** Dropdown under the filter button plus a backdrop that closes it on any outside tap. */
export function FilterOverlay({
  controller,
  position,
}: {
  controller: BackpackController;
  position: { top: number; left?: number; right?: number };
}) {
  const { state, dispatch } = controller;
  if (!state.filterMenuOpen) {
    return null;
  }
  return (
    <>
      <Pressable
        accessibilityLabel={t('BACKPACK_FILTER')}
        style={styles.backdrop}
        onPress={() => dispatch({ type: 'closeFilterMenu' })}
      />
      <View style={[styles.menu, position]}>
        <FilterMenu
          value={state.filter}
          onChange={filter => dispatch({ type: 'filter', filter })}
        />
      </View>
    </>
  );
}

const styles = StyleSheet.create({
  back: {
    position: 'absolute',
    width: u(32),
    height: u(32),
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 2,
  },
  backdrop: {
    ...StyleSheet.absoluteFill,
    zIndex: 4,
    backgroundColor: 'transparent',
  },
  menu: {
    position: 'absolute',
    zIndex: 5,
    backgroundColor: colors.purple550,
    borderRadius: u(8),
  },
});
