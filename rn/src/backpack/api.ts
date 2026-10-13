import type { GodotCommand } from '../GodotBridge';
import type { AvatarState } from '../events';
import type { Wearable } from './catalog';
import type { Emote } from './emotes';
import type { CameraFocus } from './taxonomy';
import { liveApi } from './liveApi';

/** Same shape as the `focus_camera` GodotCommand, typed with the backpack's own CameraFocus. */
export type FocusCameraCommand = { type: 'focus_camera'; target: CameraFocus };

export type BackpackCommand = GodotCommand | FocusCameraCommand;

export type Catalog = { wearables: Wearable[]; emotes: Emote[] };

/** Everything the Backpack needs from the outside; `mockApi` and `liveApi` implement it. */
export interface BackpackApi {
  loadCatalog(): Promise<Catalog>;
  send(command: BackpackCommand): void;
  /** Latest equipped avatar, or null until Godot reports it. */
  useAvatar(): AvatarState | null;
  openMarketplace(): void;
}

let active: BackpackApi = liveApi;

export const setBackpackApi = (api: BackpackApi): void => {
  active = api;
};

export const getBackpackApi = (): BackpackApi => active;
