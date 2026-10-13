import { StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, layout, LinearFill, radii } from '../ds';
import { AvatarHole, StageBackground } from './AvatarStage';
import {
  BackButton,
  FilterOverlay,
  HeaderRow,
  LayoutProps,
} from './BackpackChrome';
import { EmoteSlots } from './EmoteSlots';
import { SheetContent } from './SheetContent';
import { u } from '../ds/scale';

const PANEL_WIDTH = 312 + 48;
const FILTER_RIGHT = 349;
const FILTER_GAP = 19;

/** Avatar on the left, tabs / slots / grid on the right; no sheet and no bottom navigation. */
export function LandscapeLayout({
  controller,
  copy,
  subRow,
  onBack,
}: LayoutProps) {
  const insets = useSafeAreaInsets();
  const { state, dispatch, catalog } = controller;
  const emotes = state.mode === 'emotes';
  const top = insets.top + u(16);
  const slotsReady = emotes && catalog.status === 'ready';

  return (
    <View style={styles.root}>
      <StageBackground />
      <AvatarHole style={[styles.hole, { marginTop: top + u(48) }]} />
      <View
        pointerEvents="box-none"
        style={[styles.header, { top, left: u(119) }]}
      >
        <HeaderRow controller={controller} filterGap={FILTER_GAP} />
      </View>
      {onBack ? (
        <BackButton onPress={onBack} style={{ left: u(52), top }} />
      ) : null}
      <View
        style={[
          styles.right,
          {
            paddingRight: Math.max(insets.right, u(30)),
            paddingTop: insets.top + u(12),
          },
        ]}
      >
        {slotsReady && (
          <View style={styles.slotColumn}>
            <EmoteSlots
              vertical
              slots={controller.emoteSlots}
              emotes={catalog.emotes}
              selected={state.emoteSlot}
              flash={state.equippedToast}
              onSelect={slot => dispatch({ type: 'emoteSlot', slot })}
            />
          </View>
        )}
        <View style={[styles.panel, emotes && styles.panelFilled]}>
          {emotes ? (
            <LinearFill
              stops={[
                { offset: 0, color: colors.purple450 },
                { offset: 1, color: colors.purple500 },
              ]}
            />
          ) : null}
          <SheetContent
            controller={controller}
            copy={copy}
            subRow={subRow}
            bottomInset={u(16)}
          />
        </View>
      </View>
      <FilterOverlay
        controller={controller}
        position={{ top: top + u(36), left: u(FILTER_RIGHT) - u(164) }}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    ...StyleSheet.absoluteFill,
    flexDirection: 'row',
    backgroundColor: colors.purple500,
  },
  hole: { flex: 1 },
  header: { position: 'absolute' },
  right: { flexDirection: 'row', gap: u(8) },
  slotColumn: { marginTop: u(48), flexShrink: 0 },
  panel: { width: u(PANEL_WIDTH) },
  panelFilled: {
    marginTop: u(10),
    borderTopLeftRadius: radii.sheet,
    borderTopRightRadius: radii.sheet,
    overflow: 'hidden',
    paddingTop: layout.gap,
  },
});
