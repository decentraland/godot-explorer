import { useSyncExternalStore } from 'react';
import type { ModalApi } from './ModalProvider';

let current: ModalApi | null = null;

/** The provider registers itself so code outside React (a menu action) can raise a modal. */
export function registerModalApi(api: ModalApi | null): void {
  current = api;
}

export function modalApi(): ModalApi | null {
  return current;
}

let showing = false;
let held = false;
const listeners = new Set<() => void>();
const urgentListeners = new Set<() => void>();
const notify = () => listeners.forEach(listener => listener());

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

/** The provider reports whether a dialog is on screen. */
export function setModalShowing(next: boolean): void {
  if (next !== showing) {
    showing = next;
    notify();
  }
}

export const isModalShowing = (): boolean => showing;

export const useModalShowing = (): boolean =>
  useSyncExternalStore(subscribe, () => showing);

/** While held (the controls overlay is up), only urgent modals show; the rest wait their turn. */
export function setModalsHeld(next: boolean): void {
  if (next !== held) {
    held = next;
    notify();
  }
}

export const useModalsHeld = (): boolean =>
  useSyncExternalStore(subscribe, () => held);

/** Called when an urgent modal (crash, ban, lost connection) is asked for. */
export function onUrgentModal(listener: () => void): () => void {
  urgentListeners.add(listener);
  return () => {
    urgentListeners.delete(listener);
  };
}

export function announceUrgentModal(): void {
  urgentListeners.forEach(listener => listener());
}
