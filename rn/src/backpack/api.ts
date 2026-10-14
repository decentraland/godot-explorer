import type { AvatarState, Control } from '../core/native';
import type { Wearable } from './catalog';
import type { Emote } from './emotes';
import { liveApi } from './liveApi';

export type Catalog = { wearables: Wearable[]; emotes: Emote[] };

/** Everything the Backpack needs from the outside; `mockApi` and `liveApi` implement it. */
export interface BackpackApi {
  loadCatalog(): Promise<Catalog>;
  send(command: Control): void;
  /** Latest equipped avatar, or null until the core reports it. */
  useAvatar(): AvatarState | null;
  openMarketplace(): void;
}

let active: BackpackApi = liveApi;

export const setBackpackApi = (api: BackpackApi): void => {
  active = api;
};

export const getBackpackApi = (): BackpackApi => active;
