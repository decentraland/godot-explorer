import { Platform } from 'react-native';
import { core } from '../core';

export const mainRealm = (): string => core().urls().mainRealm;
export const placesApi = (): string => core().urls().places;
export const eventsApi = (): string => core().urls().eventsApi;
export const destinationsApi = (): string => core().urls().destinationsApi;

// mobile-bff decides what Discover lists: the global `banned` tag, and on iOS (with this tag)
// the block list of casinos, A/R-rated places and `banned_ios`.
const bffFilter = (): string =>
  Platform.OS === 'ios' ? 'sdk=7&tag=allowed_ios' : 'sdk=7';
export const destinationsUrl = (query: string): string =>
  `${destinationsApi()}/?${query}&${bffFilter()}`;
export const bffEventsUrl = (query?: string): string =>
  `${core().urls().mobileEventsApi}/?${bffFilter()}${query ? `&${query}` : ''}`;

export const featuredUrl = (): string =>
  destinationsUrl('offset=0&limit=12&tag=featured');
export const mostActiveUrl = (): string =>
  destinationsUrl('offset=0&limit=12&order_by=most_active');

/** What the Discover cards and the detail sheet show, whether it came from a place or an event. */
export type Place = {
  id: string;
  title: string;
  creator: string;
  image: string;
  users: number;
  likeRate: number | null;
  /** `x,y` for a parcel, or the world name. */
  location: string;
  /** Where Jump In goes: the realm URL and the parcel to spawn at. */
  realm: string;
  x: number;
  y: number;
  world?: boolean;
  description?: string;
  categories?: string[];
};

export type DiscoverEvent = {
  id: string;
  title: string;
  creator: string;
  image: string;
  live: boolean;
  trending: boolean;
  attendees: number;
  startsAt: number;
  place: Place;
  endsAt?: number;
  recurrence?: 'once' | 'daily' | 'weekly' | 'monthly';
  description?: string;
  /** Upcoming occurrences for the schedule rows of the expanded sheet. */
  schedule?: { start: number; end: number }[];
  featured?: boolean;
};

export const worldRealm = (name: string): string =>
  `${core().urls().worldsContentServer}${name.toLowerCase()}`;

const parseParcel = (position: string): [number, number] => {
  const [x, y] = position.split(',').map(Number);
  return [isFinite(x) ? x : 0, isFinite(y) ? y : 0];
};

const str = (value: unknown, fallback = ''): string =>
  typeof value === 'string' ? value : fallback;
const num = (value: unknown, fallback = 0): number =>
  typeof value === 'number' && isFinite(value) ? value : fallback;

type Raw = Record<string, unknown>;
const rows = (payload: unknown): Raw[] => {
  const data = (payload as { data?: unknown } | null)?.data;
  return Array.isArray(data)
    ? (data.filter(item => typeof item === 'object' && item !== null) as Raw[])
    : [];
};

export function parsePlaces(payload: unknown): Place[] {
  return rows(payload).flatMap(raw => {
    const id = str(raw.id);
    const title = str(raw.title);
    if (!id || !title) {
      return [];
    }
    const isWorld = raw.world === true;
    const worldName = str(raw.world_name);
    const position = str(raw.base_position);
    const [x, y] = isWorld ? [0, 0] : parseParcel(position);
    return [
      {
        id,
        title,
        creator: str(raw.contact_name) || str(raw.owner),
        image: str(raw.image),
        users: num(raw.user_count),
        likeRate: typeof raw.like_rate === 'number' ? raw.like_rate : null,
        location: isWorld ? worldName : position,
        realm: isWorld && worldName ? worldRealm(worldName) : mainRealm(),
        x,
        y,
        world: isWorld,
        description: str(raw.description) || undefined,
        categories: Array.isArray(raw.categories)
          ? raw.categories.filter((c): c is string => typeof c === 'string')
          : undefined,
      },
    ];
  });
}

const recurrenceOf = (raw: Raw): NonNullable<DiscoverEvent['recurrence']> => {
  const frequency = str(raw.recurrent_frequency).toLowerCase();
  return raw.recurrent === true &&
    (frequency === 'daily' || frequency === 'monthly')
    ? frequency
    : raw.recurrent === true
    ? 'weekly'
    : 'once';
};

export function parseEvents(payload: unknown): DiscoverEvent[] {
  return rows(payload).flatMap(raw => {
    const id = str(raw.id);
    const title = str(raw.name);
    if (!id || !title) {
      return [];
    }
    const attendees = num(raw.total_attendees);
    const world = raw.world === true;
    return [
      {
        id,
        title,
        creator: str(raw.user_name),
        image: str(raw.image),
        live: raw.live === true,
        trending: raw.trending === true,
        attendees,
        startsAt: Date.parse(str(raw.start_at)) || 0,
        endsAt: Date.parse(str(raw.finish_at)) || undefined,
        recurrence: recurrenceOf(raw),
        description: str(raw.description) || undefined,
        place: {
          id,
          title,
          creator: str(raw.user_name),
          image: str(raw.image),
          users: attendees,
          likeRate: null,
          location: world ? str(raw.server) : `${num(raw.x)},${num(raw.y)}`,
          realm:
            world && str(raw.server)
              ? worldRealm(str(raw.server))
              : mainRealm(),
          x: world ? 0 : num(raw.x),
          y: world ? 0 : num(raw.y),
          world,
        },
      },
    ];
  });
}

// Live and trending, then live, then the rest by their next start (no date last).
const eventRank = (raw: Raw): number =>
  raw.live === true ? (raw.trending === true ? 0 : 1) : 2;
const byNextStart = (a: Raw, b: Raw): number => {
  const [x, y] = [str(a.next_start_at), str(b.next_start_at)];
  return x && y ? (x < y ? -1 : x > y ? 1 : 0) : x ? -1 : y ? 1 : 0;
};

/** An events listing as the Discover rails show it: approved events only, live ones first. */
export function parseListedEvents(payload: unknown): DiscoverEvent[] {
  const data = rows(payload)
    .filter(raw => raw.approved === true)
    .sort(
      (a, b) =>
        eventRank(a) - eventRank(b) ||
        (eventRank(a) === 2 ? byNextStart(a, b) : 0),
    );
  return parseEvents({ data });
}

export async function getJson(
  url: string,
  fetchImpl: typeof fetch,
): Promise<unknown> {
  const response = await fetchImpl(url);
  if (!response.ok) {
    throw new Error(`${url} responded ${response.status}`);
  }
  return response.json();
}

export type DiscoverData = {
  featured: Place[];
  events: DiscoverEvent[];
  mostActive: Place[];
};

/** A missing events service doesn't fail the page. */
export async function fetchDiscover(
  fetchImpl: typeof fetch = fetch,
): Promise<DiscoverData> {
  const [featured, mostActive, events] = await Promise.all([
    getJson(featuredUrl(), fetchImpl),
    getJson(mostActiveUrl(), fetchImpl),
    getJson(bffEventsUrl(), fetchImpl).catch(() => null),
  ]);
  return {
    featured: parsePlaces(featured),
    mostActive: parsePlaces(mostActive),
    events: parseListedEvents(events),
  };
}
