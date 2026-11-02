import { CameraMode, TutorialStep } from '../core/native';
import type { FtueTuning, HudSnapshot } from '../core/native';

/** Metres one HUD report (4 a second) may add to the walk; a longer jump is a teleport. */
const MAX_REPORT_METERS = 10;
/** Lets the menu finish opening before the next highlight is drawn. */
export const MENU_OPEN_MS = 250;

/** Steps 1-3 hide the RN HUD and point at the engine's controls. */
export const GESTURE_STEPS: readonly TutorialStep[] = [
  TutorialStep.Move,
  TutorialStep.Camera,
  TutorialStep.Pinch,
];

type Sample = { x: number; z: number; yaw: number; pitch: number };

export type Run = {
  step: TutorialStep;
  replay: boolean;
  /** The player did what the step pointed at: its hint is gone and they use the thing freely. */
  engaged: boolean;
  /** The step is done; `step` starts at `at` (ms). */
  next?: { step: TutorialStep; at: number };
  /** Every touch is swallowed until then, so a stray tap cannot close the chat just opened. */
  guardUntil?: number;
  moved: number;
  rotated: number;
  last?: Sample;
  mode?: CameraMode;
  seenFirst: boolean;
  seenThird: boolean;
  cameraLocked: boolean;
  menuOpen: boolean;
  ended?: 'completed' | 'skipped';
};

export type Fact =
  | { type: 'hud'; hud: HudSnapshot; now: number }
  | { type: 'chat'; open: boolean; now: number }
  | { type: 'menu'; open: boolean; social: boolean; now: number }
  | { type: 'backpack'; open: boolean }
  | { type: 'equipped' }
  | { type: 'tick'; now: number }
  /** Back in the world after a load: the step starts over. */
  | { type: 'reenter' };

export function startRun(replay: boolean): Run {
  return enter(
    {
      step: TutorialStep.Move,
      replay,
      engaged: false,
      moved: 0,
      rotated: 0,
      seenFirst: false,
      seenThird: false,
      cameraLocked: false,
      menuOpen: false,
    },
    TutorialStep.Move,
  );
}

function enter(run: Run, step: TutorialStep): Run {
  // A scene that fixes the camera mode makes pinching do nothing.
  if (step === TutorialStep.Pinch && run.cameraLocked) {
    return enter(run, TutorialStep.Chat);
  }
  return {
    ...run,
    step,
    engaged: false,
    next: undefined,
    guardUntil: undefined,
    moved: 0,
    rotated: 0,
    last: undefined,
    mode: undefined,
    seenFirst: false,
    seenThird: false,
  };
}

const advance = (run: Run, step: TutorialStep, at: number): Run =>
  run.next ? run : { ...run, next: { step, at } };

/** Shortest turn between two headings, in degrees. */
const turn = (from: number, to: number) => {
  const d = (((to - from) % 360) + 540) % 360;
  return Math.abs(d - 180);
};

function onHud(
  run: Run,
  hud: HudSnapshot,
  now: number,
  tuning: FtueTuning,
): Run {
  const sample = {
    x: hud.position.x,
    z: hud.position.z,
    yaw: hud.cameraYaw,
    pitch: hud.cameraPitch,
  };
  const prev = run.last;
  let next: Run = { ...run, last: sample, cameraLocked: hud.cameraModeLocked };
  if (run.next) {
    return next;
  }
  const delay = now + tuning.stepDelaySeconds * 1000;
  switch (run.step) {
    case TutorialStep.Move: {
      const walked = prev
        ? Math.hypot(sample.x - prev.x, sample.z - prev.z)
        : 0;
      const moved = run.moved + (walked < MAX_REPORT_METERS ? walked : 0);
      next = { ...next, moved };
      return moved >= tuning.moveMeters
        ? advance(next, TutorialStep.Camera, delay)
        : next;
    }
    case TutorialStep.Camera: {
      const rotated =
        run.rotated +
        (prev
          ? turn(prev.yaw, sample.yaw) + Math.abs(sample.pitch - prev.pitch)
          : 0);
      next = { ...next, rotated };
      return rotated >= tuning.cameraDegrees
        ? advance(next, TutorialStep.Pinch, delay)
        : next;
    }
    case TutorialStep.Pinch: {
      const changed = run.mode !== undefined && hud.cameraMode !== run.mode;
      next = {
        ...next,
        mode: hud.cameraMode,
        seenFirst:
          run.seenFirst ||
          (changed && hud.cameraMode === CameraMode.FirstPerson),
        seenThird:
          run.seenThird ||
          (changed && hud.cameraMode === CameraMode.ThirdPerson),
      };
      return next.seenFirst && next.seenThird
        ? advance(next, TutorialStep.Chat, delay)
        : next;
    }
    default:
      return next;
  }
}

export function reduceRun(run: Run, fact: Fact, tuning: FtueTuning): Run {
  if (run.ended) {
    return run;
  }
  switch (fact.type) {
    case 'hud':
      return onHud(run, fact.hud, fact.now, tuning);
    case 'chat':
      if (run.step !== TutorialStep.Chat) {
        return run;
      }
      if (fact.open && !run.engaged) {
        return {
          ...run,
          engaged: true,
          guardUntil: fact.now + tuning.chatSeconds * 1000,
        };
      }
      if (!fact.open && run.engaged) {
        // Closed too soon to count as seen (a sent command, the system back): ask again.
        if (run.guardUntil !== undefined && fact.now < run.guardUntil) {
          return enter(run, TutorialStep.Chat);
        }
        // The menu button also closes the chat: the player already did the menu step.
        return enter(
          run,
          run.menuOpen ? TutorialStep.Social : TutorialStep.Menu,
        );
      }
      return run;
    case 'menu': {
      const next = { ...run, menuOpen: fact.open };
      if (!fact.open) {
        return next;
      }
      const opened = fact.now + MENU_OPEN_MS;
      if (run.step === TutorialStep.Chat && run.engaged) {
        return advance(next, TutorialStep.Social, opened);
      }
      if (run.step === TutorialStep.Menu) {
        return advance({ ...next, engaged: true }, TutorialStep.Social, opened);
      }
      if (run.step === TutorialStep.Social && !run.engaged && fact.social) {
        return advance(
          { ...next, engaged: true },
          TutorialStep.Backpack,
          fact.now + tuning.socialSeconds * 1000,
        );
      }
      return next;
    }
    case 'backpack':
      if (fact.open && run.step === TutorialStep.Backpack) {
        return enter(run, TutorialStep.Equip);
      }
      if (!fact.open && run.step === TutorialStep.Equip) {
        // The Backpack has no skip button: leaving it without equipping is the way out.
        return { ...run, ended: run.engaged ? 'completed' : 'skipped' };
      }
      return run;
    case 'equipped':
      return run.step === TutorialStep.Equip ? { ...run, engaged: true } : run;
    case 'tick':
      if (run.next && fact.now >= run.next.at) {
        return enter(run, run.next.step);
      }
      if (run.guardUntil !== undefined && fact.now >= run.guardUntil) {
        return { ...run, guardUntil: undefined };
      }
      return run;
    case 'reenter':
      return enter(
        run,
        run.step === TutorialStep.Equip ? TutorialStep.Backpack : run.step,
      );
  }
}

/** When the run next needs a `tick`, if ever. */
export function nextTick(run: Run): number | undefined {
  const times = [run.next?.at, run.guardUntil].filter(
    (t): t is number => t !== undefined,
  );
  return times.length ? Math.min(...times) : undefined;
}
