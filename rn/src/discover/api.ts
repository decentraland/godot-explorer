export const MAIN_REALM = 'https://realm-provider-ea.decentraland.org/main';
const WORLDS = 'https://worlds-content-server.decentraland.org/world';
const PLACES = 'https://places.decentraland.org/api/places';
const EVENTS = 'https://events.decentraland.org/api/events';

export const FEATURED_URL = `${PLACES}?only_highlighted=true&limit=12&order_by=like_score`;
export const MOST_ACTIVE_URL = `${PLACES}?limit=12&order_by=most_active&order=desc`;
export const LIVE_EVENTS_URL = `${EVENTS}?list=live&limit=10`;
export const UPCOMING_EVENTS_URL = `${EVENTS}?list=upcoming&limit=10`;

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
};

export const worldRealm = (name: string): string =>
  `${WORLDS}/${name.toLowerCase()}`;

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
        realm: isWorld && worldName ? worldRealm(worldName) : MAIN_REALM,
        x,
        y,
      },
    ];
  });
}

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
        place: {
          id,
          title,
          creator: str(raw.user_name),
          image: str(raw.image),
          users: attendees,
          likeRate: null,
          location: world ? str(raw.server) : `${num(raw.x)},${num(raw.y)}`,
          realm:
            world && str(raw.server) ? worldRealm(str(raw.server)) : MAIN_REALM,
          x: world ? 0 : num(raw.x),
          y: world ? 0 : num(raw.y),
        },
      },
    ];
  });
}

async function getJson(url: string, fetchImpl: typeof fetch): Promise<unknown> {
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

/** Live events first, then the upcoming ones; a missing events service doesn't fail the page. */
export async function fetchDiscover(
  fetchImpl: typeof fetch = fetch,
): Promise<DiscoverData> {
  const [featured, mostActive, live, upcoming] = await Promise.all([
    getJson(FEATURED_URL, fetchImpl),
    getJson(MOST_ACTIVE_URL, fetchImpl),
    getJson(LIVE_EVENTS_URL, fetchImpl).catch(() => null),
    getJson(UPCOMING_EVENTS_URL, fetchImpl).catch(() => null),
  ]);
  return {
    featured: parsePlaces(featured),
    mostActive: parsePlaces(mostActive),
    events: [...parseEvents(live), ...parseEvents(upcoming)],
  };
}
