import { ControlsFtueElement, SceneEntryFtue } from '../native';
import type { FtueApi } from '../api/ftue';

const ALL = [
  ControlsFtueElement.Menu,
  ControlsFtueElement.Chat,
  ControlsFtueElement.Movement,
  ControlsFtueElement.Emotes,
];

/** The core's rules in miniature: an account marked new gets the tutorial until it answers, the
 * others get the controls they have not seen. Nothing outlives the process. */
export function createMockFtue(): FtueApi {
  let shown: ControlsFtueElement[] = [];
  let newPlayer = false;
  let offered = false;
  return {
    ftueTuning: () => ({
      overlayLockSeconds: 3,
      overlayTapThrough: false,
      moveMeters: 1,
      cameraDegrees: 30,
      socialSeconds: 3,
      stepDelaySeconds: 2,
      chatSeconds: 3,
    }),
    ftueSceneEntered: blocked => {
      if (blocked) {
        return new SceneEntryFtue.Nothing();
      }
      if (newPlayer && !offered) {
        return new SceneEntryFtue.Tutorial();
      }
      const elements = ALL.filter(e => !shown.includes(e));
      return elements.length
        ? new SceneEntryFtue.Controls({ elements })
        : new SceneEntryFtue.Nothing();
    },
    controlsFtueDismissed: elements => {
      shown = [...new Set([...shown, ...elements])];
      offered = true;
    },
    tutorialWelcomeAnswered: start => {
      shown = ALL;
      offered = offered || !start;
    },
    tutorialStepEntered: () => {},
    tutorialSkipped: () => {
      offered = true;
    },
    tutorialCompleted: () => {
      offered = true;
      shown = ALL;
    },
    tutorialReplayRequested: () => {},
    ftueMarkNewPlayer: () => {
      newPlayer = true;
      offered = false;
    },
  };
}
