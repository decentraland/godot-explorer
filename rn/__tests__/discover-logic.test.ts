import type { DiscoverEvent, Place } from '../src/discover/api';
import {
  calendarUrl,
  eventJumpLink,
  formatDuration,
  formatSchedule,
  placeJumpLink,
  recurrenceKey,
} from '../src/discover/eventTime';
import { isOffline, RailState } from '../src/discover/rails';
import {
  historyMatches,
  initialSearch,
  normalizeKeyword,
  parseCoordinates,
  searchReducer,
  SEARCH_DEBOUNCE_MS,
} from '../src/discover/search';
import { eventMeta, placeStats } from '../src/discover/stats';

const place = (over: Partial<Place> = {}): Place => ({
  id: 'p',
  title: 'T',
  creator: 'c',
  image: '',
  users: 3,
  likeRate: 0.9,
  location: '-3,-2',
  realm: 'r',
  x: -3,
  y: -2,
  ...over,
});

describe('placeStats', () => {
  it('shows all three when the data exists', () => {
    const s = placeStats(place());
    expect(s.layout).toBe('thirds');
    expect(s.cells.map(c => c.kind)).toEqual(['users', 'likes', 'location']);
  });
  it('drops users when nobody is online and puts the location first', () => {
    const s = placeStats(place({ users: 0 }));
    expect(s.layout).toBe('ends');
    expect(s.cells.map(c => c.kind)).toEqual(['location', 'likes']);
  });
  it('keeps only the location when votes and users are missing', () => {
    const s = placeStats(place({ users: 0, likeRate: null }));
    expect(s.cells.map(c => c.kind)).toEqual(['location']);
  });
  it('uses the globe and the world name for worlds', () => {
    const s = placeStats(
      place({ users: 0, world: true, location: 'me.dcl.eth' }),
    );
    expect(s.cells[0]).toMatchObject({ icon: 'globe', text: 'me.dcl.eth' });
  });
  it('is empty when there is nothing to show', () => {
    expect(
      placeStats(place({ users: 0, likeRate: null, location: '' })).cells,
    ).toEqual([]);
  });
});

describe('eventMeta', () => {
  const event = (over: Partial<DiscoverEvent> = {}): DiscoverEvent => ({
    id: 'e',
    title: 'E',
    creator: 'c',
    image: '',
    live: false,
    trending: false,
    attendees: 0,
    startsAt: 0,
    recurrence: 'weekly',
    endsAt: 3_600_000,
    place: place(),
    ...over,
  });
  it('lists recurrence, duration and location', () => {
    const cells = eventMeta(event());
    expect(cells.map(c => c.icon)).toEqual(['calendar', 'clock', 'pin']);
    expect(cells[0].text).toBe('Weekly');
    expect(cells[1].text).toBe('1 Hr');
  });
  it('omits the location cell when there is none and globes worlds', () => {
    expect(eventMeta(event({ place: place({ location: '' }) }))).toHaveLength(
      2,
    );
    expect(
      eventMeta(event({ place: place({ world: true, location: 'w' }) }))[2]
        .icon,
    ).toBe('globe');
  });
});

describe('eventTime', () => {
  it('formats durations', () => {
    expect(formatDuration(30 * 60_000)).toBe('30 Min');
    expect(formatDuration(3_600_000)).toBe('1 Hr');
    expect(formatDuration(5_400_000)).toBe('1 Hr 30 Min');
    expect(formatDuration(0)).toBeNull();
  });
  it('maps recurrence to a label key', () => {
    expect(recurrenceKey('daily')).toBe('EVENT_FREQUENCY_DAILY');
    expect(recurrenceKey(undefined)).toBe('EVENT_FREQUENCY_ONCE');
  });
  it('formats the schedule line in the given offset', () => {
    const start = Date.UTC(2026, 0, 19, 19, 10);
    const line = formatSchedule(start, start + 2 * 3_600_000, 180);
    expect(line).toBe('Mon, Jan 19. 04:10pm to 06:10pm (UTC-3)');
  });
  it('builds the calendar link and the jump links', () => {
    const url = calendarUrl({
      title: 'A b',
      description: 'd',
      location: 'Decentraland at 1,2',
      start: Date.UTC(2026, 0, 1, 10),
      end: Date.UTC(2026, 0, 1, 11),
    });
    expect(url).toContain('text=A%20b');
    expect(url).toContain('dates=20260101T100000Z%2F20260101T110000Z');
    expect(placeJumpLink(place())).toContain('position=-3%2C-2');
    expect(placeJumpLink(place({ world: true, location: 'a.eth' }))).toContain(
      'realm=a.eth',
    );
    expect(eventJumpLink('x')).toContain('id=x');
  });
});

describe('search helpers', () => {
  it('parses coordinates', () => {
    expect(parseCoordinates('44,-48')).toEqual({ x: 44, y: -48 });
    expect(parseCoordinates(' -35 , -31 ')).toEqual({ x: -35, y: -31 });
    expect(parseCoordinates('44,-')).toBeNull();
    expect(parseCoordinates('casa')).toBeNull();
  });
  it('normalises keywords', () => {
    expect(normalizeKeyword("  Casa!?* 'Azul' ")).toBe('Casa Azul');
  });
  it('matches history by containment, max four', () => {
    const history = ['Casa', 'Casa Azul', 'Bar', 'Casa Py', 'Casas', 'Casa Z'];
    expect(historyMatches(history, 'ca')).toEqual([
      'Casa',
      'Casa Azul',
      'Casa Py',
      'Casas',
    ]);
    expect(historyMatches(history, '')).toEqual(history.slice(0, 4));
  });
});

describe('searchReducer', () => {
  const results = { featured: [], events: [], mostActive: [], favorites: [] };
  it('walks idle -> suggest -> loading -> results', () => {
    let s = initialSearch(['Casa']);
    expect(s.phase).toBe('idle');
    s = searchReducer(s, { type: 'change', text: 'Ca' });
    expect(s.phase).toBe('suggest');
    s = searchReducer(s, { type: 'submit', text: 'Casa' });
    expect(s).toMatchObject({ phase: 'loading', query: 'Casa' });
    s = searchReducer(s, {
      type: 'results',
      results: { ...results, featured: [place()] },
    });
    expect(s.phase).toBe('results');
    s = searchReducer(s, { type: 'change', text: '' });
    expect(s.phase).toBe('idle');
  });
  it('is empty when nothing matched and errors on failure', () => {
    const loading = searchReducer(initialSearch([]), {
      type: 'submit',
      text: 'x',
    });
    expect(searchReducer(loading, { type: 'results', results }).phase).toBe(
      'empty',
    );
    expect(searchReducer(loading, { type: 'fail' }).phase).toBe('error');
  });
  it('keeps suggestions and clears history', () => {
    let s = searchReducer(initialSearch(['a']), {
      type: 'change',
      text: 'abc',
    });
    s = searchReducer(s, {
      type: 'suggestions',
      items: [{ kind: 'keyword', text: 'abc' }],
    });
    expect(s.suggestions).toHaveLength(1);
    s = searchReducer(s, { type: 'clearHistory' });
    expect(s.history).toEqual([]);
  });
  it('exports the debounce', () => expect(SEARCH_DEBOUNCE_MS).toBe(400));
});

describe('isOffline', () => {
  const err: RailState<unknown> = { status: 'error' };
  const ready: RailState<unknown> = {
    status: 'ready',
    items: [],
    hasMore: false,
  };
  it('needs every primary rail to fail', () => {
    expect(isOffline([err, err, err])).toBe(true);
    expect(isOffline([err, ready, err])).toBe(false);
    expect(isOffline([])).toBe(false);
  });
});
