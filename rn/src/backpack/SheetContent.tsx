import { useCallback } from 'react';
import { View } from 'react-native';
import { ColorPanel } from './ColorPanel';
import { ConnectionLost } from './ConnectionLost';
import { EmotesGrid } from './EmotesPanel';
import { BackpackController } from './useBackpack';
import { WearablesPanel } from './WearablesPanel';

/** What lives inside the sheet (portrait) or right panel (landscape) for the current mode. */
export function SheetContent({
  controller,
  bottomInset = 0,
}: {
  controller: BackpackController;
  bottomInset?: number;
}) {
  const { state, dispatch, catalog, avatar, slot } = controller;
  const target = state.colorTarget;
  const setColor = controller.setColor(target ?? 'skin');
  const closeColor = useCallback(
    () => dispatch({ type: 'closeColor' }),
    [dispatch],
  );

  if (catalog.status === 'error') {
    return <ConnectionLost onRetry={controller.retry} />;
  }

  if (target && avatar && state.mode === 'wearables') {
    return (
      <ColorPanel
        target={target}
        value={avatar[target]}
        onChange={setColor}
        onBack={closeColor}
      />
    );
  }

  if (state.mode === 'emotes') {
    return catalog.status === 'ready' ? (
      <EmotesGrid
        emotes={catalog.emotes}
        slots={controller.emoteSlots}
        bottomInset={bottomInset}
        onPress={controller.pressEmote}
      />
    ) : (
      <View />
    );
  }

  return (
    <WearablesPanel
      catalog={catalog.status === 'ready' ? catalog.wearables : null}
      avatar={avatar}
      tab={state.tab}
      slot={slot}
      filter={state.filter}
      bottomInset={bottomInset}
      onTab={tab => dispatch({ type: 'tab', tab })}
      onSlot={next => dispatch({ type: 'slot', slot: next })}
      onToggle={controller.toggleWearable}
      onOpenColor={entry =>
        entry.color && dispatch({ type: 'openColor', target: entry.color })
      }
    />
  );
}
