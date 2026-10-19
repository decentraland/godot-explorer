import { useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { BottomSheet, colors, LinearFill, snapHeights } from '../ds';
import type { SnapConfig } from '../ds';
import { AvatarHole, StageBackground } from './AvatarStage';
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
import { u } from '../ds/scale';

/** Sheet heights as fractions of the area above the footer (Figma 720x1600 frames). */
const WEARABLES_SNAPS: SnapConfig = { peek: 24, half: 0.514, full: 0.863 };
const EMOTES_SNAPS: SnapConfig = { peek: 24, half: 0.427, full: 0.863 };
const SLOTS_GAP = u(12);

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
  const config = emotes ? EMOTES_SNAPS : WEARABLES_SNAPS;
  const heights = snapHeights(area, config);
  const covered = state.sheet === 'full';
  const top = Math.max(insets.top + u(4), u(43));

  return (
    <View style={styles.root}>
      <View
        style={styles.area}
        onLayout={event => setArea(event.nativeEvent.layout.height)}
      >
        <StageBackground bottom={heights[state.sheet]} />
        <View style={[styles.header, { paddingTop: top }]}>
          {onBack ? (
            <BackButton onPress={onBack} style={{ left: u(18), top }} />
          ) : null}
          <HeaderRow controller={controller} />
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
          handleWidth={100}
          handlePadding={7}
          background={
            <LinearFill
              stops={[
                { offset: 0, color: colors.purple450 },
                { offset: 1, color: colors.purple550 },
              ]}
            />
          }
        >
          <SheetContent
            controller={controller}
            copy={copy}
            subRow={subRow}
            bottomInset={heights.full - heights[state.sheet]}
          />
        </BottomSheet>
        <FilterOverlay
          controller={controller}
          position={{ top: top + u(36), right: u(24) }}
        />
      </View>
      {footer}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
  area: { flex: 1 },
  header: { paddingLeft: u(80), paddingRight: u(24) },
  hole: { flex: 1 },
  slots: { position: 'absolute', left: 0, right: 0 },
});
