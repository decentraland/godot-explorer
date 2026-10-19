import { useCallback, useEffect, useMemo, useReducer, useState } from 'react';
import { CameraFocus, Control } from '../core/native';
import { BackpackApi, getBackpackApi } from './api';
import {
  activeSlot,
  BackpackState,
  initialBackpack,
  reduceBackpack,
} from './backpackState';
import { Wearable } from './catalog';
import { Emote, EMOTE_SLOTS } from './emotes';
import { tapEmote } from './emoteLogic';
import { wearableCommand } from './selection';
import { cameraFocusFor, ColorTarget, coreColorTarget } from './taxonomy';
import { CatalogState, useCatalog } from './useCatalog';

const TOAST_MS = 900;
const BUSY_MS = 3000;

export type BackpackOptions = {
  api?: BackpackApi;
  initial?: Partial<BackpackState>;
};

/** Everything a Backpack layout needs: UI state, loaded catalog, avatar and the actions on it. */
export function useBackpack(options: BackpackOptions = {}) {
  const api = options.api ?? getBackpackApi();
  const [state, dispatch] = useReducer(reduceBackpack, {
    ...initialBackpack,
    ...options.initial,
  });
  const { catalog, retry } = useCatalog(api);
  const avatar = api.useAvatar();
  const holdToast = options.initial?.equippedToast === true;
  const [busy, setBusy] = useState<string | null>(null);

  const focus =
    state.mode === 'emotes' ? CameraFocus.Overall : cameraFocusFor(state.tab);
  useEffect(() => {
    api.send(new Control.FocusCamera({ focus }));
  }, [api, focus]);

  // The avatar event is the engine's answer to an equip/unequip: it ends the pending dim.
  useEffect(() => setBusy(null), [avatar]);
  useEffect(() => {
    if (busy === null) {
      return undefined;
    }
    const timer = setTimeout(() => setBusy(null), BUSY_MS);
    return () => clearTimeout(timer);
  }, [busy]);

  useEffect(() => {
    // A toast the screen starts with (previews) stays up.
    if (!state.equippedToast || holdToast) {
      return undefined;
    }
    const timer = setTimeout(() => dispatch({ type: 'toastDone' }), TOAST_MS);
    return () => clearTimeout(timer);
  }, [state.equippedToast, holdToast]);

  const toggleWearable = useCallback(
    (item: Wearable, equipped: boolean) => {
      const command = wearableCommand(item, equipped);
      if (command) {
        setBusy(item.id);
        api.send(command);
      }
    },
    [api],
  );

  const setColor = useCallback(
    (target: ColorTarget) => (color: string) =>
      api.send(
        new Control.SetColor({ target: coreColorTarget(target), color }),
      ),
    [api],
  );

  const emoteSlots = useMemo(
    () => avatar?.emotes ?? Array.from({ length: EMOTE_SLOTS }, () => ''),
    [avatar],
  );

  const pressEmote = useCallback(
    (emote: Emote) => {
      const tap = tapEmote(
        emote,
        emoteSlots,
        state.emoteSlot,
        state.pendingEmote,
      );
      tap.commands.forEach(command => api.send(command));
      if (tap.equippedSlot !== null) {
        dispatch({ type: 'emoteEquipped', slot: tap.equippedSlot });
      } else {
        dispatch({ type: 'emotePending', id: tap.pending });
      }
    },
    [api, emoteSlots, state.emoteSlot, state.pendingEmote],
  );

  return {
    state,
    dispatch,
    catalog: catalog as CatalogState,
    retry,
    avatar,
    busy,
    slot: activeSlot(state),
    emoteSlots,
    toggleWearable,
    setColor,
    pressEmote,
    openMarketplace: api.openMarketplace,
  };
}

export type BackpackController = ReturnType<typeof useBackpack>;
