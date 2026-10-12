import { ReactNode, useEffect } from 'react';
import { Pressable, StyleSheet, View, useWindowDimensions } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, Icon, LinearFill } from '../ds';
import { setOrientation } from '../GodotBridge';
import { AvatarHole, StageBackground } from './AvatarStage';
import { BackpackHeader } from './BackpackHeader';
import { EmoteSlotsRow } from './EmotesPanel';
import { FilterMenu } from './FilterMenu';
import { Sheet } from './Sheet';
import { SheetContent } from './SheetContent';
import { BackpackController, useBackpack } from './useBackpack';

const SHEET_RATIO = 0.455;
const LEFT_RATIO = 0.54;

export type BackpackScreenProps = {
  /** Portrait bar under the sheet: the app's bottom navigation, or a Continue button. */
  footer: ReactNode;
  /** Whether the screen may rotate to landscape; the entry flow stays portrait. */
  allowLandscape?: boolean;
  onBack?: () => void;
};

export function BackpackScreen({
  footer,
  allowLandscape = true,
  onBack,
}: BackpackScreenProps) {
  const { width, height } = useWindowDimensions();
  const controller = useBackpack();

  useEffect(() => {
    setOrientation(allowLandscape ? 'sensor' : 'portrait');
    return () => setOrientation('portrait');
  }, [allowLandscape]);

  return width > height ? (
    <Landscape controller={controller} onBack={onBack} />
  ) : (
    <Portrait controller={controller} footer={footer} onBack={onBack} />
  );
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
      <View style={{ paddingTop: insets.top + 12 }}>
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
        position={{ top: insets.top + 12 + 36, right: 24 }}
      />
    </View>
  );
}

function Landscape({
  controller,
  onBack,
}: {
  controller: BackpackController;
  onBack?: () => void;
}) {
  const { width } = useWindowDimensions();
  const insets = useSafeAreaInsets();
  const { state, dispatch, catalog } = controller;
  return (
    <View style={styles.root}>
      <StageBackground />
      <View style={styles.landscape}>
        <View
          style={[
            styles.left,
            { paddingLeft: insets.left + 12, paddingTop: insets.top + 10 },
          ]}
        >
          <Header controller={controller} onBack={onBack} />
          <AvatarHole style={styles.hole} />
          {state.mode === 'emotes' && catalog.status === 'ready' && (
            <EmoteSlotsRow
              slots={controller.emoteSlots}
              emotes={catalog.emotes}
              selected={state.emoteSlot}
              onSelect={slot => dispatch({ type: 'emoteSlot', slot })}
            />
          )}
        </View>
        <View
          style={[
            styles.right,
            { paddingRight: insets.right, paddingTop: insets.top + 6 },
          ]}
        >
          <LinearFill
            stops={[
              { offset: 0, color: colors.purple450 },
              { offset: 1, color: colors.purple550 },
            ]}
          />
          <SheetContent controller={controller} bottomInset={insets.bottom} />
        </View>
      </View>
      <FilterOverlay
        controller={controller}
        position={{
          top: insets.top + 10 + 36,
          left: Math.round(width * LEFT_RATIO) - 165 - 16,
        }}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
  headerWrap: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
  },
  back: {
    position: 'absolute',
    left: 18,
    width: 32,
    height: 32,
    alignItems: 'center',
    justifyContent: 'center',
  },
  hole: { flex: 1 },
  menu: { position: 'absolute', zIndex: 5 },
  landscape: { flex: 1, flexDirection: 'row' },
  left: { flex: LEFT_RATIO },
  right: {
    flex: 1 - LEFT_RATIO,
    overflow: 'hidden',
    backgroundColor: colors.purple500,
  },
});
