import { ReactNode } from 'react';
import { Pressable, StyleSheet, View, useWindowDimensions } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, Icon } from '../ds';
import { AvatarHole, StageBackground } from './AvatarStage';
import { BackpackHeader } from './BackpackHeader';
import { EmoteSlotsRow } from './EmotesPanel';
import { FilterMenu } from './FilterMenu';
import { Sheet } from './Sheet';
import { SheetContent } from './SheetContent';
import { BackpackController, useBackpack } from './useBackpack';
import { u } from '../ds/scale';

const SHEET_RATIO = 0.455;

export type BackpackScreenProps = {
  /** Bar under the sheet: the app's bottom navigation, or a Continue button. */
  footer: ReactNode;
  /** Figma's backpack has no back button; the entry flow needs one. */
  onBack?: () => void;
};

/** Portrait only: everything before a scene is portrait-locked, so there is no landscape layout. */
export function BackpackScreen({ footer, onBack }: BackpackScreenProps) {
  const controller = useBackpack();
  return <Portrait controller={controller} footer={footer} onBack={onBack} />;
}

function Header({
  controller,
  onBack,
}: {
  controller: BackpackController;
  onBack?: () => void;
}) {
  const { state, dispatch } = controller;
  return (
    <View style={styles.headerWrap}>
      {onBack ? (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Back"
          onPress={onBack}
          hitSlop={12}
          style={styles.back}
        >
          <Icon name="back" size={22} />
        </Pressable>
      ) : null}
      <BackpackHeader
        mode={state.mode}
        filterActive={state.filter !== 'all' || state.filterMenuOpen}
        onMode={mode => dispatch({ type: 'mode', mode })}
        onFilter={() => dispatch({ type: 'toggleFilterMenu' })}
      />
    </View>
  );
}

function FilterOverlay({
  controller,
  position,
}: {
  controller: BackpackController;
  position: { top: number; right?: number; left?: number };
}) {
  const { state, dispatch } = controller;
  if (!state.filterMenuOpen) {
    return null;
  }
  return (
    <View style={[styles.menu, position]}>
      <FilterMenu
        value={state.filter}
        onChange={filter => dispatch({ type: 'filter', filter })}
      />
    </View>
  );
}

function Portrait({
  controller,
  footer,
  onBack,
}: {
  controller: BackpackController;
  footer: ReactNode;
  onBack?: () => void;
}) {
  const { height } = useWindowDimensions();
  const insets = useSafeAreaInsets();
  const { state, dispatch, catalog } = controller;
  return (
    <View style={styles.root}>
      <StageBackground />
      <View style={{ paddingTop: insets.top + u(12) }}>
        <Header controller={controller} onBack={onBack} />
      </View>
      <AvatarHole style={styles.hole} />
      {state.mode === 'emotes' && catalog.status === 'ready' && (
        <EmoteSlotsRow
          slots={controller.emoteSlots}
          emotes={catalog.emotes}
          selected={state.emoteSlot}
          onSelect={slot => dispatch({ type: 'emoteSlot', slot })}
        />
      )}
      <Sheet
        height={Math.round(height * SHEET_RATIO)}
        state={state.sheet}
        onState={sheet => dispatch({ type: 'sheet', sheet })}
      >
        <SheetContent controller={controller} />
      </Sheet>
      {footer}
      <FilterOverlay
        controller={controller}
        position={{ top: insets.top + u(12) + u(36), right: u(24) }}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
  headerWrap: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'flex-end',
    paddingHorizontal: u(24),
  },
  back: {
    position: 'absolute',
    left: u(18),
    width: u(32),
    height: u(32),
    alignItems: 'center',
    justifyContent: 'center',
  },
  hole: { flex: 1 },
  menu: { position: 'absolute', zIndex: 5 },
});
