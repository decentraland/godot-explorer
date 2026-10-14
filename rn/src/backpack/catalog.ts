import { core } from '../core';
import { Rarity } from '../ds';

export const BASE_PREFIX = 'urn:decentraland:off-chain:base-avatars:';
export const MALE = `${BASE_PREFIX}BaseMale`;
export const FEMALE = `${BASE_PREFIX}BaseFemale`;

export const wearablesUrl = (): string =>
  `${
    core().urls().peerLambdas
  }collections/wearables?collectionId=urn:decentraland:off-chain:base-avatars`;
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

const RARITIES: readonly string[] = [
  'common',
  'uncommon',
  'rare',
  'epic',
  'legendary',
  'exotic',
  'mythic',
  'unique',
];

type RawWearable = {
  id?: unknown;
  name?: unknown;
  thumbnail?: unknown;
  rarity?: unknown;
  i18n?: unknown;
  data?: { category?: unknown; representations?: unknown };
};

function englishName(raw: RawWearable): string {
  const entries = Array.isArray(raw.i18n) ? raw.i18n : [];
  const en = entries.find(
    (entry): entry is { code: string; text: string } =>
      typeof entry === 'object' &&
      entry !== null &&
      entry.code === 'en' &&
      typeof entry.text === 'string',
  );
  return en?.text ?? (typeof raw.name === 'string' ? raw.name : '');
}

function bodyShapesOf(raw: RawWearable): string[] {
  const reps = Array.isArray(raw.data?.representations)
    ? raw.data.representations
    : [];
  const shapes = reps.flatMap((rep: { bodyShapes?: unknown }) =>
    Array.isArray(rep?.bodyShapes)
      ? rep.bodyShapes.filter((s): s is string => typeof s === 'string')
      : [],
  );
  return Array.from(new Set(shapes));
}

/** Validates the lambda payload; entries without id, category or thumbnail are dropped. */
export function parseWearables(payload: unknown): Wearable[] {
  const list = (payload as { wearables?: unknown } | null)?.wearables;
  if (!Array.isArray(list)) {
    return [];
  }
  return list.flatMap((raw: RawWearable) => {
    const category = raw?.data?.category;
    if (
      typeof raw?.id !== 'string' ||
      typeof category !== 'string' ||
      typeof raw.thumbnail !== 'string'
    ) {
      return [];
    }
    const rarity: Rarity =
      typeof raw.rarity === 'string' && RARITIES.includes(raw.rarity)
        ? (raw.rarity as Rarity)
        : 'base';
    return [
      {
        id: raw.id,
        name: englishName(raw),
        category,
        thumbnail: raw.thumbnail,
        rarity,
        bodyShapes: bodyShapesOf(raw),
        collectible: !raw.id.startsWith(BASE_PREFIX),
      },
    ];
  });
}

export async function fetchWearables(
  fetchImpl: typeof fetch = fetch,
): Promise<Wearable[]> {
  const response = await fetchImpl(wearablesUrl());
  if (!response.ok) {
    throw new Error(`wearables catalog responded ${response.status}`);
  }
  const wearables = parseWearables(await response.json());
  if (wearables.length === 0) {
    throw new Error('wearables catalog is empty');
  }
  return wearables;
}
