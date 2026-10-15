import { core } from '../core';
import { Rarity } from '../ds';
import { contentsUrl, entitiesUrl } from './catalog';

export const BASE_EMOTE_PREFIX = 'urn:decentraland:off-chain:base-emotes:';
export const EMOTE_SLOTS = 10;

export type Emote = {
  id: string;
  urn: string;
  name: string;
  thumbnail?: string;
  /** Owned emotes carry a rarity; base emotes are not collectible. */
  rarity?: Rarity;
  collectible?: boolean;
};

/** The emotes every account has, from the core. */
export const DEFAULT_EMOTES: Emote[] = core().baseEmotes();

/** Profiles store base emotes either as the bare id or as the full urn. */
export function emoteIdOf(urnOrId: string): string {
  return urnOrId.startsWith(BASE_EMOTE_PREFIX)
    ? urnOrId.slice(BASE_EMOTE_PREFIX.length)
    : urnOrId;
}

type Entity = {
  pointers?: string[];
  content?: { file: string; hash: string }[];
  metadata?: { thumbnail?: string };
};

/** Attaches thumbnail URLs from the content entities; emotes without one keep `thumbnail` unset. */
export function withThumbnails(emotes: Emote[], entities: unknown): Emote[] {
  const byPointer = new Map<string, string>();
  for (const entity of Array.isArray(entities) ? (entities as Entity[]) : []) {
    const file = entity?.metadata?.thumbnail;
    const hash = entity?.content?.find(entry => entry.file === file)?.hash;
    if (hash && entity.pointers?.[0]) {
      byPointer.set(entity.pointers[0].toLowerCase(), contentsUrl() + hash);
    }
  }
  return emotes.map(emote => ({
    ...emote,
    thumbnail: byPointer.get(emote.urn.toLowerCase()),
  }));
}

export async function fetchEmoteThumbnails(
  fetchImpl: typeof fetch = fetch,
): Promise<Emote[]> {
  const response = await fetchImpl(entitiesUrl(), {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ pointers: DEFAULT_EMOTES.map(emote => emote.urn) }),
  });
  if (!response.ok) {
    throw new Error(`emote entities responded ${response.status}`);
  }
  return withThumbnails(DEFAULT_EMOTES, await response.json());
}
