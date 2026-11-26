import type { AvatarState, Control } from '../core/native';
import type { Wearable } from './catalog';
import type { Emote } from './emotes';
import { liveApi } from './liveApi';

export type Catalog = { wearables: Wearable[]; emotes: Emote[] };

/** Everything the Backpack needs from the outside; `mockApi` and `liveApi` implement it. */
export interface BackpackApi {
  loadCatalog(): Promise<Catalog>;
  /** What the account owns on top of the catalog; reloaded each time the Backpack opens. */
  loadOwned(): Promise<Catalog>;
  /** Publishes the avatar the Backpack leaves on the player; false when nothing changed. */
  saveAvatar(): Promise<boolean>;
  send(command: Control): void;
  /** Latest equipped avatar, or null until the core reports it. */
  useAvatar(): AvatarState | null;
  openMarketplace(): void;
  /** The user's credits balance, for the iOS in-app purchase badge. */
  loadCredits(): Promise<number>;
}

let active: BackpackApi = liveApi;

export const setBackpackApi = (api: BackpackApi): void => {
  active = api;
};

export const getBackpackApi = (): BackpackApi => active;
