import { core } from '../core';
import type { CatalogItem } from '../core/native';
import { Rarity } from '@dcl/ui-native';

export const BASE_PREFIX = 'urn:decentraland:off-chain:base-avatars:';
export const MALE = `${BASE_PREFIX}BaseMale`;
export const FEMALE = `${BASE_PREFIX}BaseFemale`;

export const entitiesUrl = (): string =>
  `${core().urls().peerContent}entities/active`;
export const contentsUrl = (): string =>
  `${core().urls().peerContent}contents/`;

export type Wearable = {
  id: string;
  name: string;
  category: string;
  thumbnail: string;
  rarity: Rarity;
  bodyShapes: string[];
  collectible: boolean;
};

export const toWearable = (item: CatalogItem): Wearable => ({
  id: item.urn,
  name: item.name,
  category: item.category,
  thumbnail: item.thumbnail,
  rarity: item.rarity as Rarity,
  bodyShapes: item.bodyShapes,
  collectible: item.collectible,
});

/** The base wearables the core fetched and now resolves avatar edits against. */
export async function loadWearables(): Promise<Wearable[]> {
  return (await core().loadAvatarCatalog()).map(toWearable);
}
