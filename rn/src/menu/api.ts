import { MAIN_REALM } from '../discover/api';
import { godotAvailable } from '../GodotBridge';
import { createBridgeMenuApi } from './bridgeApi';
import { createMockApi } from './mockApi';
import type { MenuApi } from './types';

let active: MenuApi | null = null;

/** Tests and the gallery install the mock; the app picks the bridge or, without the engine, the mock. */
export function setMenuApi(api: MenuApi | null): void {
  active = api;
}

export function getMenuApi(): MenuApi {
  if (!active) {
    active = godotAvailable
      ? createBridgeMenuApi(createMockApi())
      : createMockApi();
  }
  return active;
}

export const sceneLink = (query: string): string =>
  `https://mobile.dclexplorer.com/open?${query}`;

/** `position=x,y` in Genesis City, `realm=<world>` otherwise. */
export function linkQueryFor(realm: string, x: number, y: number): string {
  if (realm === MAIN_REALM) {
    return `position=${x},${y}`;
  }
  return `realm=${realm.split('/').pop() ?? ''}`;
}
