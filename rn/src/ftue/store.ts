import { useSyncExternalStore } from 'react';
import { core } from '../core';
import type { ControlsFtueElement, FtueTuning } from '../core/native';
import { onUrgentModal, setModalsHeld } from '../modals/bridge';

/** What the world shows of the first-time guidance; the core decided it, the shell draws it. */
export type FtueState = {
  /** The controls overlay is up, labelling these. */
  controls: ControlsFtueElement[] | null;
  /** This scene entry was checked; the welcome chat line waits for it. */
  decided: boolean;
};

const NONE: FtueState = { controls: null, decided: false };

let state: FtueState = NONE;
let tuning: FtueTuning | null = null;
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

/** A world is loading: what the last entry showed goes, unanswered. */
export function leaveEntry(): void {
  stopUrgent?.();
  stopUrgent = undefined;
  set({ ...state, controls: null, decided: false });
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

export function resetFtueForTests(): void {
  stopUrgent?.();
  stopUrgent = undefined;
  tuning = null;
  state = NONE;
  setModalsHeld(false);
  listeners.clear();
}
