import {
  ComponentProps,
  memo,
  startTransition,
  useCallback,
  useDeferredValue,
  useEffect,
  useState,
} from 'react';
import { StyleSheet, View } from 'react-native';
import type { SubRow } from './BackpackChrome';
import { ColorEditor } from './ColorEditor';
import { EmotesGrid } from './EmotesGrid';
import { activeSlot } from './backpackState';
import { SkeletonGrid } from './GridStates';
import type { EmptyCopy } from './platformPolicy';
import { visibleEmotes } from './selection';
import { BackpackController } from './useBackpack';
import { KeptLayer } from './KeptLayer';
import { WearablesPanel } from './WearablesPanel';
import { colorKey, TABS, TabKey } from './taxonomy';

/** What lives inside the sheet (portrait) or right panel (landscape) for the current mode. */
export function SheetContent({
  controller,
  copy,
  subRow,
  columns,
  bottomInset = 0,
  flushTop = false,
}: {
  controller: BackpackController;
  copy: EmptyCopy;
  subRow?: SubRow;
  columns?: number;
  bottomInset?: number;
  /** Portrait emotes: the sheet has no handle and already spaces the grid. */
  flushTop?: boolean;
}) {
  const { state, dispatch, catalog, avatar } = controller;
  // The tab row answers a tap at once; the new tab's grid follows in a render that can wait.
  const gridTab = useDeferredValue(state.tab);
  const target = state.colorTarget;
  const editing = !!(target && avatar && state.mode === 'wearables');
  const wearablesShown = state.mode === 'wearables' && !editing;
  const emotesShown = state.mode === 'emotes';
  // The grids need the avatar too: without it All has no worn items to put first and lists both
  // body shapes, so the tiles moved when it arrived (opening from Discover, before any preview).
  const ready = catalog.status === 'ready' && avatar !== null;
  // Once the sheet is up, the other tabs' grids and the emotes grid mount hidden at low priority:
  // changing tab or mode then shows a grid whose cards and images are already there.
  const [kept, setKept] = useState(false);
  useEffect(() => {
    const frame = requestAnimationFrame(() =>
      startTransition(() => setKept(true)),
    );
    return () => cancelAnimationFrame(frame);
  }, []);
  const closeColor = useCallback(
    () => dispatch({ type: 'closeColor' }),
    [dispatch],
  );

  const panel = (tab: TabKey) => {
    const hidden = !wearablesShown || tab !== gridTab;
    return (
      <KeptLayer key={tab} shown={tab === gridTab}>
        <WearablesPanel
          hidden={hidden}
          catalog={ready ? catalog.wearables : null}
          avatar={avatar}
          tab={tab}
          activeTab={state.tab}
          slot={activeSlot({ ...state, tab })}
          filter={state.filter}
          columns={columns}
          bottomInset={bottomInset}
          busy={controller.busy}
          copy={copy}
          subRow={subRow}
          onTab={next => dispatch({ type: 'tab', tab: next })}
          onSlot={next => dispatch({ type: 'slot', slot: next })}
          onToggle={controller.toggleWearable}
          onColor={(color, next) => controller.setColor(color)(next)}
          onOpenColor={next => dispatch({ type: 'openColor', target: next })}
          onMarketplace={controller.openMarketplace}
        />
      </KeptLayer>
    );
  };

  return (
    <View style={styles.fill}>
      <KeptLayer shown={wearablesShown}>
        {TABS.map(entry => entry.key)
          .filter(tab => tab === gridTab || kept)
          .map(panel)}
      </KeptLayer>
      {(emotesShown || kept) && (
        <KeptLayer shown={emotesShown}>
          {ready ? (
            <KeptEmotes
              hidden={!emotesShown}
              emotes={visibleEmotes(catalog.emotes, state.filter)}
              slots={controller.emoteSlots}
              selected={state.emoteSlot}
              pending={state.pendingEmote}
              columns={columns}
              bottomInset={bottomInset}
              copy={copy}
              flushTop={flushTop}
              onPress={controller.pressEmote}
              onMarketplace={controller.openMarketplace}
            />
          ) : (
            <SkeletonGrid columns={columns} />
          )}
        </KeptLayer>
      )}
      {editing && target && avatar ? (
        <View style={StyleSheet.absoluteFill}>
          <ColorEditor
            target={target}
            value={avatar[colorKey(target)]}
            onChange={controller.setColor(target)}
            onBack={closeColor}
            bottomInset={bottomInset}
          />
        </View>
      ) : null}
    </View>
  );
}

/** The emotes grid, kept mounted behind the wearables: hidden, it skips renders. */
const KeptEmotes = memo(
  ({
    hidden: _hidden,
    ...props
  }: ComponentProps<typeof EmotesGrid> & { hidden: boolean }) => (
    <EmotesGrid {...props} />
  ),
  (prev, next) => prev.hidden && next.hidden,
);

const styles = StyleSheet.create({
  fill: { flex: 1 },
});
