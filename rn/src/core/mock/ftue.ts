import { ControlsFtueElement, SceneEntryFtue } from '../native';
import type { FtueApi } from '../api/ftue';

const ALL = [
  ControlsFtueElement.Menu,
  ControlsFtueElement.Chat,
  ControlsFtueElement.Movement,
  ControlsFtueElement.Emotes,
];

/** The core's rule in miniature: each control is labelled until the overlay is dismissed. Nothing
 * outlives the process. */
export function createMockFtue(): FtueApi {
  let shown: ControlsFtueElement[] = [];
  return {
    ftueTuning: () => ({ overlayLockSeconds: 3, overlayTapThrough: false }),
    ftueSceneEntered: blocked => {
      const elements = ALL.filter(e => !shown.includes(e));
      return blocked || !elements.length
        ? new SceneEntryFtue.Nothing()
        : new SceneEntryFtue.Controls({ elements });
    },
    controlsFtueDismissed: elements => {
      shown = [...new Set([...shown, ...elements])];
    },
  };
}
