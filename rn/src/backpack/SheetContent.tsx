import { useCallback } from 'react';
import type { SubRow } from './BackpackChrome';
import { ColorEditor } from './ColorEditor';
import { EmotesGrid } from './EmotesGrid';
import { SkeletonGrid } from './GridStates';
import type { EmptyCopy } from './platformPolicy';
import { visibleEmotes } from './selection';
import { BackpackController } from './useBackpack';
import { WearablesPanel } from './WearablesPanel';
import { bridgeTarget } from './taxonomy';

/** What lives inside the sheet (portrait) or right panel (landscape) for the current mode. */
export function SheetContent({
  controller,
  copy,
  subRow,
  columns,
  bottomInset = 0,
}: {
  controller: BackpackController;
  copy: EmptyCopy;
  subRow?: SubRow;
  columns?: number;
  bottomInset?: number;
}) {
  const { state, dispatch, catalog, avatar, slot } = controller;
  const target = state.colorTarget;
  const closeColor = useCallback(
    () => dispatch({ type: 'closeColor' }),
    [dispatch],
  );

  if (target && avatar && state.mode === 'wearables') {
    return (
      <ColorEditor
        target={target}
        value={avatar[bridgeTarget(target)]}
        onChange={controller.setColor(target)}
        onBack={closeColor}
        bottomInset={bottomInset}
      />
    );
  }

  if (state.mode === 'emotes') {
    return catalog.status === 'ready' ? (
      <EmotesGrid
        emotes={visibleEmotes(catalog.emotes, state.filter)}
        slots={controller.emoteSlots}
        selected={state.emoteSlot}
        pending={state.pendingEmote}
        columns={columns}
        bottomInset={bottomInset}
        copy={copy}
        onPress={controller.pressEmote}
        onMarketplace={controller.openMarketplace}
      />
    ) : (
      <SkeletonGrid columns={columns} />
    );
  }

  return (
    <WearablesPanel
      catalog={catalog.status === 'ready' ? catalog.wearables : null}
      avatar={avatar}
      tab={state.tab}
      slot={slot}
      filter={state.filter}
      columns={columns}
      bottomInset={bottomInset}
      busy={controller.busy}
      copy={copy}
      subRow={subRow}
      onTab={tab => dispatch({ type: 'tab', tab })}
      onSlot={next => dispatch({ type: 'slot', slot: next })}
      onToggle={controller.toggleWearable}
      onColor={(color, next) => controller.setColor(color)(next)}
      onOpenColor={next => dispatch({ type: 'openColor', target: next })}
      onMarketplace={controller.openMarketplace}
    />
  );
}
