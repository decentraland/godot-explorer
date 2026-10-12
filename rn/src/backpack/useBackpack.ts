import { useCallback, useEffect, useMemo, useReducer } from 'react';
import { ensureEngine, send, useEngine } from '../engine';
import { Wearable } from './catalog';
import { activeSlot, initialBackpack, reduceBackpack } from './backpackState';
import { CatalogState, useCatalog } from './useCatalog';
import { Emote, EMOTE_SLOTS, emoteIdOf } from './emotes';
import { targetEmoteSlot } from './selection';
import { ColorTarget } from './taxonomy';

/** Everything a Backpack layout needs: UI state, loaded catalog, avatar and the actions on it. */
export function useBackpack() {
  const [state, dispatch] = useReducer(reduceBackpack, initialBackpack);
  const { catalog, retry } = useCatalog();
  const { avatar } = useEngine();

  useEffect(() => {
    ensureEngine();
  }, []);

  const toggleWearable = useCallback((item: Wearable, equipped: boolean) => {
    if (item.category === 'body_shape') {
      send({ type: 'set_body_shape', urn: item.id });
    } else {
      send({ type: equipped ? 'unequip' : 'equip', urn: item.id });
    }
  }, []);

  const setColor = useCallback(
    (target: ColorTarget) => (color: string) =>
      send({ type: 'set_color', target, color }),
    [],
  );

  const emoteSlots = useMemo(
    () => avatar?.emotes ?? Array.from({ length: EMOTE_SLOTS }, () => ''),
    [avatar],
  );

  const pressEmote = useCallback(
    (emote: Emote, equipped: boolean) => {
      if (equipped) {
        const slot = emoteSlots.findIndex(
          urn => urn !== '' && emoteIdOf(urn) === emote.id,
        );
        send({ type: 'set_emote_slot', slot, urn: '' });
        return;
      }
      send({ type: 'play_emote', urn: emote.urn });
      const slot = targetEmoteSlot(emoteSlots, state.emoteSlot);
      send({ type: 'set_emote_slot', slot, urn: emote.urn });
      dispatch({
        type: 'emoteSlot',
        slot: Math.min(slot + 1, EMOTE_SLOTS - 1),
      });
    },
    [emoteSlots, state.emoteSlot],
  );

  return {
    state,
    dispatch,
    catalog: catalog as CatalogState,
    retry,
    avatar,
    slot: activeSlot(state),
    emoteSlots,
    toggleWearable,
    setColor,
    pressEmote,
  };
}

export type BackpackController = ReturnType<typeof useBackpack>;
