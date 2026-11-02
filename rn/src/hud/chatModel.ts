import type { ChatLine, ChatProfile } from './types';

export const MAX_REDUCED_CHARS = 80;
const WORLD_LIMIT = 150;
const MAX_URL_LENGTH = 2048;

export type Segment = {
  kind: 'text' | 'mention' | 'coord' | 'world' | 'url';
  text: string;
  /** Mention name, `x,y`, world name or the https url to open. */
  value?: string;
  mine?: boolean;
};

export type TokenizeContext = {
  /** Display names (`Name#tag` or claimed `Name`) of the avatars nearby. */
  names: string[];
  myName?: string;
};

const NICK_COLORS = [
  '#FF7439',
  '#73D3D3',
  '#FFD700',
  '#34CE76',
  '#438FFF',
  '#A14BF3',
  '#FF8362',
  '#CAFF73',
  '#FEA217',
  '#81D1FF',
  '#F6C1FF',
  '#73FFAF',
] as const;

export function splitName(full: string): {
  nickname: string;
  tag: string;
  claimed: boolean;
} {
  const parts = full.split('#').filter(Boolean);
  if (parts.length > 1) {
    return { nickname: parts[0], tag: `#${parts[1]}`, claimed: false };
  }
  return { nickname: full, tag: '', claimed: true };
}

/** Stand-in for `DclAvatar.get_nickname_color` until Godot sends the colour with the profile. */
export function nameColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i += 1) {
    hash = (hash * 31 + name.charCodeAt(i)) % 2 ** 32;
  }
  return NICK_COLORS[hash % NICK_COLORS.length];
}

export function truncateReduced(text: string): string {
  return text.length > MAX_REDUCED_CHARS
    ? `${text.slice(0, MAX_REDUCED_CHARS)}...`
    : text;
}

const DANGEROUS = [
  // eslint-disable-next-line no-script-url
  'javascript:',
  'data:',
  'vbscript:',
  'file:',
  'about:',
  '<script',
  'onclick',
  'onerror',
  'onload',
  '%3cscript',
  '&#',
];

export function isSafeUrl(url: string): boolean {
  if (!url.startsWith('https://') || url.length > MAX_URL_LENGTH) {
    return false;
  }
  const lower = url.toLowerCase();
  if (DANGEROUS.some(pattern => lower.includes(pattern))) {
    return false;
  }
  if (/[<>"'`{}[\]]/.test(url)) {
    return false;
  }
  return !url.slice(8).includes('//');
}

export const escapeUrlForLink = (url: string): string =>
  url.replace(/\[/g, '%5B').replace(/\]/g, '%5D');

const toHttps = (url: string): string =>
  url.startsWith('https://') ? url : `https://${url.replace(/^http:\/\//, '')}`;

type Hit = { start: number; end: number; segment: Segment };

const baseNameLower = (full: string): string =>
  full.startsWith('#')
    ? ''
    : (full.split('#').filter(Boolean)[0] ?? '').toLowerCase();

/** Every lowercased text a sender may type after `@` -> the identity to show. The bare name of an unclaimed profile only resolves while it names one nearby profile. */
function mentionLookup(ctx: TokenizeContext): Map<string, string> {
  const identities = new Set(ctx.names.filter(Boolean));
  if (ctx.myName) {
    identities.add(ctx.myName);
  }
  const owners = new Map<string, number>();
  for (const identity of identities) {
    const base = baseNameLower(identity);
    if (base) {
      owners.set(base, (owners.get(base) ?? 0) + 1);
    }
  }
  const lookup = new Map<string, string>();
  for (const identity of identities) {
    lookup.set(identity.toLowerCase(), identity);
    const base = baseNameLower(identity);
    if (base && owners.get(base) === 1) {
      lookup.set(base, identity);
    }
  }
  return lookup;
}

const escapeRegex = (text: string): string =>
  text.replace(/[\\^$.*+?()[\]{}|]/g, '\\$&');

function mentionHits(text: string, ctx: TokenizeContext): Hit[] {
  if (!text.includes('@')) {
    return [];
  }
  const lookup = mentionLookup(ctx);
  if (lookup.size === 0) {
    return [];
  }
  // Longest first so "Lara Fire" wins over a nearby "Lara".
  const names = [...lookup.keys()]
    .sort((a, b) => b.length - a.length)
    .map(escapeRegex);
  const re = new RegExp(
    `(?<![\\p{L}\\p{N}_.-])@(${names.join('|')})(?![\\p{L}\\p{N}_#])`,
    'giu',
  );
  const hits: Hit[] = [];
  for (const m of text.matchAll(re)) {
    const identity = lookup.get(m[1].toLowerCase());
    if (!identity) {
      continue;
    }
    const segment: Segment = {
      kind: 'mention',
      text: `@${identity}`,
      value: identity,
    };
    hits.push({
      start: m.index,
      end: m.index + m[0].length,
      segment: identity === ctx.myName ? { ...segment, mine: true } : segment,
    });
  }
  return hits;
}

function worldHits(text: string): Hit[] {
  return [...text.matchAll(/[a-zA-Z0-9][-a-zA-Z0-9]*\.dcl\.eth/g)].map(m => ({
    start: m.index,
    end: m.index + m[0].length,
    segment: { kind: 'world', text: m[0], value: m[0] },
  }));
}

function urlHits(text: string): Hit[] {
  const hits: Hit[] = [];
  const re =
    /https?:\/\/[^\s]+|www\.[^\s]+|[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}[^\s]*/g;
  for (const m of text.matchAll(re)) {
    const shown = m[0].replace(/[.,!?);:]+$/, '');
    const value = toHttps(shown);
    if (shown && isSafeUrl(value)) {
      hits.push({
        start: m.index,
        end: m.index + shown.length,
        segment: { kind: 'url', text: shown, value: escapeUrlForLink(value) },
      });
    }
  }
  return hits;
}

function coordHits(text: string): Hit[] {
  const hits: Hit[] = [];
  for (const m of text.matchAll(/-?\d+,-?\d+/g)) {
    const [x, y] = m[0].split(',').map(Number);
    if (Math.abs(x) <= WORLD_LIMIT && Math.abs(y) <= WORLD_LIMIT) {
      hits.push({
        start: m.index,
        end: m.index + m[0].length,
        segment: { kind: 'coord', text: m[0], value: m[0] },
      });
    }
  }
  return hits;
}

/** Splits a message into text and tappable parts; earlier kinds win overlaps (mention, world, url, coord). */
export function tokenize(text: string, ctx: TokenizeContext): Segment[] {
  const taken: Hit[] = [];
  const free = (hit: Hit) =>
    !taken.some(t => hit.start < t.end && t.start < hit.end);
  for (const hit of [
    ...mentionHits(text, ctx),
    ...worldHits(text),
    ...urlHits(text),
    ...coordHits(text),
  ]) {
    if (free(hit)) {
      taken.push(hit);
    }
  }
  taken.sort((a, b) => a.start - b.start);
  const out: Segment[] = [];
  let cursor = 0;
  for (const hit of taken) {
    if (hit.start > cursor) {
      out.push({ kind: 'text', text: text.slice(cursor, hit.start) });
    }
    out.push(hit.segment);
    cursor = hit.end;
  }
  if (cursor < text.length) {
    out.push({ kind: 'text', text: text.slice(cursor) });
  }
  return out.length ? out : [{ kind: 'text', text }];
}

export type RowKind = 'system' | 'own' | 'other';

export function rowKind(
  line: ChatLine,
  profile: ChatProfile | undefined,
): RowKind {
  if (line.address === 'system') {
    return 'system';
  }
  return profile?.own ? 'own' : 'other';
}
