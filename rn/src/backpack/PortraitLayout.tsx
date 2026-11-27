import { ComponentRef, useCallback, useMemo, useRef, useState } from 'react';
import { PixelRatio, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import {
  BottomSheet,
  color,
  control,
  CreditsBadge,
  snapHeights,
  space,
  u,
} from '@dcl/ui-native';
import type { SnapConfig } from '@dcl/ui-native';
import { AvatarHole, PortraitStage, StageBackground } from './AvatarStage';
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
const WEARABLES_SNAPS: SnapConfig = { peek: u(27), half: 0.531, full: 0.863 };
const EMOTES_SNAPS: SnapConfig = { peek: u(27), half: 0.427, full: 0.863 };
/** Figma `Emote Wheel`: 16 px above the sheet, half of it the row's own ring padding. */
const SLOTS_GAP = space.xs;
/** Figma `IAP-IOS-Portrait`: `Badge/Credits` 33 px above the sheet (no token). */
const CREDITS_GAP = u(16.5);
/** Figma `Header Content` gap: from the Emotes pill to the filter button. */
const FILTER_GAP = space['64'];

export function PortraitLayout({
  controller,
  copy,
  subRow,
  credits,
  footer,
  onBack,
  tools = true,
}: LayoutProps) {
  const insets = useSafeAreaInsets();
  const [area, setArea] = useState(0);
  const [slotRow, setSlotRow] = useState(0);
  const areaRef = useRef<ComponentRef<typeof View>>(null);
  const areaTop = useRef(0);
  const { state, dispatch, catalog } = controller;
  // The avatar window is sized for the snap the sheet rests at, not for the one it is heading to:
  // it keeps its size while the sheet slides over it.
  const [rest, setRest] = useState(state.sheet);
  const heading = useRef(state.sheet);
  heading.current = state.sheet;
  const emotes = state.mode === 'emotes';
  const covered = rest === 'full';
  const showSlots = emotes && catalog.status === 'ready' && !covered;
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
        {tools ? <PortraitStage /> : <StageBackground bottom={heights.peek} />}
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
          {tools ? (
            <HeaderRow controller={controller} filterGap={FILTER_GAP} />
          ) : null}
        </View>
        {covered ? (
          <View style={styles.hole} />
        ) : (
          // The engine draws over RN here, so the avatar must end above the emote chip and slots.
          <AvatarHole
            ready={area > 0}
            style={[
              styles.hole,
              {
                marginBottom:
                  heights[rest] + (showSlots ? slotRow + SLOTS_GAP : 0),
              },
            ]}
          />
        )}
        {credits != null && !emotes && !covered && (
          <CreditsBadge
            amount={credits}
            style={[
              styles.credits,
              { bottom: heights[state.sheet] + CREDITS_GAP },
            ]}
          />
        )}
        {showSlots && (
          <View
            pointerEvents="box-none"
            style={[styles.slots, { bottom: heights[state.sheet] + SLOTS_GAP }]}
            onLayout={event => setSlotRow(event.nativeEvent.layout.height)}
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
        {/* Hidden until measured: drawn before, the sheet shows a frame at the wrong height. */}
        <View
          style={[StyleSheet.absoluteFill, area === 0 && styles.unmeasured]}
          pointerEvents="box-none"
        >
          <BottomSheet
            snap={state.sheet}
            onSnapChange={sheet => dispatch({ type: 'sheet', sheet })}
            containerHeight={area || undefined}
            config={config}
            handle={!emotes}
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
              flushTop={emotes}
              entry={!tools}
            />
          </BottomSheet>
        </View>
        {tools ? (
          <FilterOverlay
            controller={controller}
            position={{
              // 8 px under the filter button, centred on the pill row.
              top: top + (control['pill-lg'] + control.sm) / 2 + space.xs,
              right: space['2xl'],
            }}
          />
        ) : null}
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
  credits: { position: 'absolute', left: space['2xl'] },
  unmeasured: { opacity: 0 },
});
