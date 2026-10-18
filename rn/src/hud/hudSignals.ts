import { useSyncExternalStore } from 'react';

/** Requests the HUD raises for screens it does not own; the shell or the menu package subscribes. */
export type HudIntent =
  | { type: 'open_menu' }
  | { type: 'open_profile'; address: string }
  | { type: 'edit_emotes' }
  | { type: 'replay_tutorial' };

const intentListeners = new Set<(intent: HudIntent) => void>();
const stateListeners = new Set<() => void>();
let menuOpen = false;
let uiHidden = false;
let chatOpen = false;

export function emitIntent(intent: HudIntent): void {
  intentListeners.forEach(listener => listener(intent));
}

export function onIntent(listener: (intent: HudIntent) => void): () => void {
  intentListeners.add(listener);
  return () => intentListeners.delete(listener);
}

/** The menu drawer calls this while open so the HUD group steps aside. */
export function setMenuOpen(open: boolean): void {
  menuOpen = open;
  stateListeners.forEach(listener => listener());
}

/** Settings/menu call this to enter Hide UI; the HUD's restore button calls it with false. */
export function setUiHidden(hidden: boolean): void {
  uiHidden = hidden;
  stateListeners.forEach(listener => listener());
}

/** The HUD reports its chat panel, so the tutorial can follow it. */
export function setChatOpen(open: boolean): void {
  if (open !== chatOpen) {
    chatOpen = open;
    stateListeners.forEach(listener => listener());
  }
}

const subscribeState = (listener: () => void) => {
  stateListeners.add(listener);
  return () => stateListeners.delete(listener);
};

export const useMenuOpen = (): boolean =>
  useSyncExternalStore(subscribeState, () => menuOpen);

export const useUiHidden = (): boolean =>
  useSyncExternalStore(subscribeState, () => uiHidden);

export const useChatOpen = (): boolean =>
  useSyncExternalStore(subscribeState, () => chatOpen);
