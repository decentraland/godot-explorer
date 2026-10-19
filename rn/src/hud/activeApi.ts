import { coreHudApi, HudApi } from './hudApi';

let active: HudApi = coreHudApi;

export const getHudApi = (): HudApi => active;

export function setHudApi(api: HudApi): void {
  active = api;
}
