import { useSyncExternalStore } from 'react';

/** Rail items; the last three leave the drawer for fullscreen pages. */
export type MenuSection =
  | 'discover'
  | 'social'
  | 'notifications'
  | 'backpack'
  | 'settings'
  | 'profile';

export type DrawerState = { open: boolean; section: MenuSection };

let state: DrawerState = { open: false, section: 'discover' };
const listeners = new Set<() => void>();

function set(next: DrawerState) {
  state = next;
  listeners.forEach(listener => listener());
}

/** The HUD avatar button calls this. */
export function openMenu(section: MenuSection = state.section): void {
  set({ open: true, section });
}

export function closeMenu(): void {
  if (state.open) {
    set({ ...state, open: false });
  }
}

export function selectSection(section: MenuSection): void {
  set({ ...state, section });
}

export function useDrawerState(): DrawerState {
  return useSyncExternalStore(
    listener => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
  );
}

export function resetDrawerForTests(): void {
  state = { open: false, section: 'discover' };
  listeners.clear();
}

type Rect = { left: number; top: number; right: number; bottom: number };
export type Zone = { left: number; top: number; width: number; height: number };

/**
 * Taps outside the panel close the drawer, except over the gamepad, which stays live.
 * `pad` is the gamepad's bounding box, `panel` the drawer's.
 */
export function closeZones(
  screen: { width: number; height: number },
  panel: Rect,
  pad: Rect,
): Zone[] {
  const zone = (left: number, top: number, right: number, bottom: number) => ({
    left,
    top,
    width: Math.max(0, right - left),
    height: Math.max(0, bottom - top),
  });
  const padLeft = Math.max(pad.left, panel.right);
  return [
    zone(0, 0, panel.left, screen.height),
    zone(panel.left, 0, panel.right, panel.top),
    zone(panel.left, panel.bottom, panel.right, screen.height),
    zone(panel.right, 0, padLeft, screen.height),
    zone(padLeft, 0, screen.width, pad.top),
  ].filter(z => z.width > 0 && z.height > 0);
}
