import { useEffect, useRef, useState } from 'react';
import { useWindowDimensions } from 'react-native';
import { core } from '../core';
import { onCoreEvents } from '../core/events';
import { SceneEntryFtue_Tags } from '../core/native';
import { useCoreState } from '../core/useCoreState';
import { useMenuOpen, useUiHidden } from '../hud/hudSignals';
import { useModalShowing } from '../modals/bridge';
import {
  entryDecided,
  ftueState,
  leaveEntry,
  showControls,
  showWelcome,
} from './store';

/**
 * Asks the core, once per scene entry, whether the controls overlay or the tutorial's welcome
 * card shows: after the loading cover (`ready`), the engine's first report of the scene and any
 * modal that was up.
 */
export function useFtueEntry(ready: boolean): void {
  const inWorld = useCoreState().tag === 'InWorld';
  const modal = useModalShowing();
  const menuOpen = useMenuOpen();
  const hidden = useUiHidden();
  const { width, height } = useWindowDimensions();
  const [reported, setReported] = useState(false);
  const asked = useRef(false);

  useEffect(() => {
    if (!inWorld) {
      asked.current = false;
      setReported(false);
      leaveEntry();
      return undefined;
    }
    return onCoreEvents({ hud: () => setReported(true) });
  }, [inWorld]);

  useEffect(() => {
    if (!ready || !inWorld || !reported || modal || asked.current) {
      return;
    }
    asked.current = true;
    const blocked =
      menuOpen || hidden || height > width || ftueState().run !== null;
    let entry;
    try {
      entry = core().ftueSceneEntered(blocked);
    } catch {
      entryDecided();
      return;
    }
    if (entry.tag === SceneEntryFtue_Tags.Controls) {
      showControls(entry.inner.elements);
    } else if (entry.tag === SceneEntryFtue_Tags.Tutorial) {
      showWelcome();
    }
    entryDecided();
  }, [ready, inWorld, reported, modal, menuOpen, hidden, width, height]);
}
