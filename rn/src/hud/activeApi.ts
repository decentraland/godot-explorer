import { godotAvailable } from '../GodotBridge';
import { bridgeHudApi, HudApi } from './hudApi';
import { simulatedHudApi } from './simulator';

let active: HudApi = godotAvailable ? bridgeHudApi : simulatedHudApi;

export const getHudApi = (): HudApi => active;

export function setHudApi(api: HudApi): void {
  active = api;
}
