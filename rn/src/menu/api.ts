import { core } from '../core';
import { createCoreMenuApi } from './coreMenuApi';
import { createMockApi } from './mockApi';
import type { MenuApi } from './types';

let active: MenuApi | null = null;

/** Tests and the gallery install the mock; the app uses the core (itself a mock without the engine). */
export function setMenuApi(api: MenuApi | null): void {
  active = api;
}

export function getMenuApi(): MenuApi {
  if (!active) {
    active = createCoreMenuApi(createMockApi());
  }
  return active;
}

export const sceneLink = (query: string): string =>
  `${core().urls().mobileOpen}?${query}`;

/** `position=x,y` in Genesis City, `realm=<world>` otherwise (the core builds the link). */
export function linkQueryFor(realm: string, x: number, y: number): string {
  return core().sceneLinkUrl(realm, x, y).split('?').slice(1).join('?');
}
