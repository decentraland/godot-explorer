import { useSyncExternalStore } from 'react';
import { core } from '../core';
import type { ControlsFtueElement, FtueTuning } from '../core/native';
import { onUrgentModal, setModalsHeld } from '../modals/bridge';
import { nextTick, reduceRun, startRun } from './tutorial';
import type { Fact, Run } from './tutorial';

/** What the world shows of the first-time guidance; the core decided it, the shell draws it. */
export type FtueState = {
  /** The controls overlay is up, labelling these. */
  controls: ControlsFtueElement[] | null;
  /** The welcome card offers the tutorial. */
  welcome: boolean;
  run: Run | null;
  /** This scene entry was checked; the welcome chat line waits for it. */
  decided: boolean;
};

const NONE: FtueState = {
  controls: null,
  welcome: false,
  run: null,
  decided: false,
};

let state: FtueState = NONE;
let tuning: FtueTuning | null = null;
let announced: Run['step'] | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;
let stopUrgent: (() => void) | undefined;
const listeners = new Set<() => void>();

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

export const useFtue = (): FtueState =>
  useSyncExternalStore(subscribe, () => state);

export const ftueState = (): FtueState => state;

/** The remote tunables, read once per process: the flags are fetched once at boot. */
export function ftueTuning(): FtueTuning {
  tuning ??= core().ftueTuning();
  return tuning;
}

function set(next: FtueState): void {
  state = next;
  setModalsHeld(next.controls !== null);
  listeners.forEach(listener => listener());
}

function setRun(run: Run | null): void {
  clearTimeout(timer);
  timer = undefined;
  if (run?.ended) {
    if (run.ended === 'completed') {
      core().tutorialCompleted(run.replay);
    } else {
      core().tutorialSkipped(run.step, run.replay);
    }
    run = null;
  }
  if (run && run.step !== announced && !run.next) {
    announced = run.step;
    core().tutorialStepEntered(run.step, run.replay);
  }
  if (!run) {
    announced = null;
  }
  const at = run ? nextTick(run) : undefined;
  if (at !== undefined) {
    timer = setTimeout(
      () => tutorialFact({ type: 'tick', now: Date.now() }),
      Math.max(0, at - Date.now()),
    );
  }
  set({ ...state, run });
}

/** A world is loading: what the last entry showed goes, unanswered. */
export function leaveEntry(): void {
  stopUrgent?.();
  stopUrgent = undefined;
  set({ ...state, controls: null, welcome: false, decided: false });
}

export function entryDecided(): void {
  set({ ...state, decided: true });
}

export function showControls(elements: ControlsFtueElement[]): void {
  stopUrgent?.();
  stopUrgent = onUrgentModal(cancelControls);
  set({ ...state, controls: elements });
}

/** The tap closed it: what it labelled is remembered. */
export function dismissControls(secondsShown: number): void {
  const elements = state.controls;
  if (elements) {
    core().controlsFtueDismissed(elements, secondsShown);
    cancelControls();
  }
}

/** Gone without an answer (a load, an urgent modal): it shows again on a later entry. */
export function cancelControls(): void {
  stopUrgent?.();
  stopUrgent = undefined;
  if (state.controls) {
    set({ ...state, controls: null });
  }
}

export function showWelcome(): void {
  set({ ...state, welcome: true });
}

export function answerWelcome(start: boolean): void {
  core().tutorialWelcomeAnswered(start);
  set({ ...state, welcome: false });
  if (start) {
    setRun(startRun(false));
  }
}

/** Settings › Gameplay › Replay: step 1 again, no welcome card. */
export function replayTutorial(): void {
  core().tutorialReplayRequested();
  if (!state.run) {
    setRun(startRun(true));
  }
}

export function skipTutorial(): void {
  if (state.run) {
    setRun({ ...state.run, ended: 'skipped' });
  }
}

export function tutorialFact(fact: Fact): void {
  if (state.run) {
    setRun(reduceRun(state.run, fact, ftueTuning()));
  }
}

export function resetFtueForTests(): void {
  clearTimeout(timer);
  stopUrgent?.();
  stopUrgent = undefined;
  tuning = null;
  announced = null;
  state = NONE;
  setModalsHeld(false);
  listeners.clear();
}
