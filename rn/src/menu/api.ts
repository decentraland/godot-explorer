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
