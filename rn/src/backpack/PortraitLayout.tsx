import { useMemo, useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import {
  BottomSheet,
  color,
  control,
  snapHeights,
  space,
  u,
} from '@dcl/ui-native';
import type { SnapConfig } from '@dcl/ui-native';
import { AvatarHole, PortraitStage } from './AvatarStage';
import {
  BackButton,
  FilterOverlay,
  HeaderRow,
  LayoutProps,
} from './BackpackChrome';
import { EmoteChip } from './EmoteChip';
import { EmoteSlots } from './EmoteSlots';
import { chipName } from './emoteLogic';
import { SheetContent } from './SheetContent';

/** Sheet heights as fractions of the area above the footer (Figma 720x1600 frames). */
/** `peek` is Figma `Sheet` Height=Collapsed: 54 px, the handle alone. */
const WEARABLES_SNAPS: SnapConfig = { peek: u(27), half: 0.514, full: 0.863 };
const EMOTES_SNAPS: SnapConfig = { peek: u(27), half: 0.427, full: 0.863 };
/** Figma `Emote Wheel`: 16 px above the sheet, half of it the row's own ring padding. */
const SLOTS_GAP = space.xs;
/** Figma `Header Content` gap: from the Emotes pill to the filter button. */
const FILTER_GAP = space['64'];

export function PortraitLayout({
  controller,
  copy,
  subRow,
  footer,
  onBack,
}: LayoutProps) {
  const insets = useSafeAreaInsets();
  const [area, setArea] = useState(0);
  const { state, dispatch, catalog } = controller;
  const emotes = state.mode === 'emotes';
  const covered = state.sheet === 'full';
  const top = Math.max(insets.top + u(4), u(43));
  // Figma `Full`: the open sheet starts right under the header (pills row + 30 px).
  const headerBottom = top + control['pill-lg'] + space['30'];
  const config = useMemo(() => {
    const base = emotes ? EMOTES_SNAPS : WEARABLES_SNAPS;
    return area > headerBottom
      ? { ...base, full: (area - headerBottom) / area }
      : base;
  }, [emotes, area, headerBottom]);
  const heights = snapHeights(area, config);

  return (
    <View style={styles.root}>
      <View
        style={styles.area}
        onLayout={event => setArea(event.nativeEvent.layout.height)}
      >
        <PortraitStage />
        <View style={[styles.header, { paddingTop: top }]}>
          {onBack ? (
            <BackButton onPress={onBack} style={{ left: u(18), top }} />
          ) : null}
          <HeaderRow controller={controller} filterGap={FILTER_GAP} />
        </View>
        {covered ? (
          <View style={styles.hole} />
        ) : (
          <AvatarHole
            style={[styles.hole, { marginBottom: heights[state.sheet] }]}
          />
        )}
        {emotes && catalog.status === 'ready' && !covered && (
          <View
            pointerEvents="box-none"
            style={[styles.slots, { bottom: heights[state.sheet] + SLOTS_GAP }]}
          >
            <EmoteChip
              name={chipName(
                catalog.emotes,
                controller.emoteSlots,
                state.emoteSlot,
                state.pendingEmote,
              )}
              equipped={state.equippedToast}
            />
            <EmoteSlots
              slots={controller.emoteSlots}
              emotes={catalog.emotes}
              selected={state.emoteSlot}
              flash={state.equippedToast}
              onSelect={slot => dispatch({ type: 'emoteSlot', slot })}
            />
          </View>
        )}
        <BottomSheet
          snap={state.sheet}
          onSnapChange={sheet => dispatch({ type: 'sheet', sheet })}
          containerHeight={area || undefined}
          config={config}
          handle={!emotes}
        >
          <SheetContent
            controller={controller}
            copy={copy}
            subRow={subRow}
            bottomInset={heights.full - heights[state.sheet]}
            flushTop={emotes}
          />
        </BottomSheet>
        <FilterOverlay
          controller={controller}
          position={{
            // 8 px under the filter button, centred on the pill row.
            top: top + (control['pill-lg'] + control.sm) / 2 + space.xs,
            right: space['2xl'],
          }}
        />
      </View>
      {footer}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: color.background.brand },
  /** Clipped so the sheet stops at the nav bar instead of running under it. */
  area: { flex: 1, overflow: 'hidden' },
  /** Figma `Header`: right-aligned, 48 px from the edge. */
  header: { paddingHorizontal: space['2xl'], alignItems: 'flex-end' },
  hole: { flex: 1 },
  slots: { position: 'absolute', left: 0, right: 0 },
});
