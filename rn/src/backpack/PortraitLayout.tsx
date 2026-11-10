import { ComponentRef, useCallback, useRef, useState } from 'react';
import { PixelRatio, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { BottomSheet, colors, snapHeights, u } from '@dcl/ui-native';
import type { SnapConfig } from '@dcl/ui-native';
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
import { core } from '../core';
import { Control } from '../core/native';

/** Sheet heights as fractions of the area above the footer (Figma 720x1600 frames). */
/** `peek` is Figma `Sheet` Height=Collapsed: 54 px, the handle alone. */
const WEARABLES_SNAPS: SnapConfig = { peek: u(27), half: 0.514, full: 0.863 };
const EMOTES_SNAPS: SnapConfig = { peek: u(27), half: 0.427, full: 0.863 };
const SLOTS_GAP = u(12);

export function PortraitLayout({
  controller,
  copy,
  subRow,
  footer,
  onBack,
  tools = true,
}: LayoutProps) {
  const insets = useSafeAreaInsets();
  const [area, setArea] = useState(0);
  const areaRef = useRef<ComponentRef<typeof View>>(null);
  const areaTop = useRef(0);
  const { state, dispatch, catalog } = controller;
  // The avatar window is sized for the snap the sheet rests at, not for the one it is heading to:
  // it keeps its size while the sheet slides over it.
  const [rest, setRest] = useState(state.sheet);
  const heading = useRef(state.sheet);
  heading.current = state.sheet;
  const emotes = state.mode === 'emotes';
  const config = emotes ? EMOTES_SNAPS : WEARABLES_SNAPS;
  const heights = snapHeights(area, config);
  const covered = rest === 'full';
  const top = Math.max(insets.top + u(4), u(43));

  // The avatar window is above every RN view: the engine cuts it at the sheet's top edge, so the
  // sheet covers the avatar while it moves, instead of the avatar showing over the sheet.
  const clip = useCallback(
    (visible: number) => {
      if (!area) {
        return;
      }
      try {
        core().control(
          new Control.PreviewClip({
            bottom: Math.round(
              (areaTop.current + area - visible) * PixelRatio.get(),
            ),
          }),
        );
      } catch {
        // The window is not open (yet): there is nothing to cut.
      }
    },
    [area],
  );

  return (
    <View style={styles.root}>
      <View
        ref={areaRef}
        style={styles.area}
        onLayout={event => {
          setArea(event.nativeEvent.layout.height);
          areaRef.current?.measureInWindow((_x, y) => {
            areaTop.current = y;
          });
        }}
      >
        <StageBackground bottom={heights.peek} />
        <View style={[styles.header, { paddingTop: top }]}>
          {onBack ? (
            // The same arrow, size and place as the avatar screen it follows: the box is 1 dp
            // larger on each side than the 30 dp arrow.
            <BackButton
              onPress={onBack}
              size={30}
              style={{ left: u(23), top: top - u(1) }}
            />
          ) : null}
          {tools ? <HeaderRow controller={controller} /> : null}
        </View>
        {covered ? (
          <View style={styles.hole} />
        ) : (
          <AvatarHole style={[styles.hole, { marginBottom: heights[rest] }]} />
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
          onMovingChange={moving => {
            if (!moving) {
              setRest(heading.current);
            }
          }}
          onVisibleChange={clip}
        >
          <SheetContent
            controller={controller}
            copy={copy}
            subRow={subRow}
            bottomInset={heights.full - heights[state.sheet]}
          />
        </BottomSheet>
        {tools ? (
          <FilterOverlay
            controller={controller}
            position={{ top: top + u(36), right: u(24) }}
          />
        ) : null}
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
