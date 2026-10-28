import { mainRealm, parseEvents, parsePlaces } from '../src/discover/api';
import {
  formatRate,
  formatUsers,
  matchesQuery,
  startsIn,
} from '../src/discover/format';

describe('parsePlaces', () => {
  it('maps places and worlds, dropping broken rows', () => {
    const places = parsePlaces({
      data: [
        {
          id: 'a',
          title: 'Genesis Plaza',
          contact_name: 'Foundation',
          image: 'i',
          user_count: 9,
          like_rate: 0.9,
          base_position: '-3,-2',
        },
        {
          id: 'b',
          title: 'My World',
          world: true,
          world_name: 'me.dcl.eth',
          like_rate: null,
        },
        { id: 'c' },
        null,
      ],
    });
    expect(places).toHaveLength(2);
    expect(places[0]).toMatchObject({
      creator: 'Foundation',
      users: 9,
      location: '-3,-2',
    });
    expect(places[1]).toMatchObject({
      location: 'me.dcl.eth',
      likeRate: null,
      users: 0,
    });
    expect(places[0]).toMatchObject({ realm: mainRealm(), x: -3, y: -2 });
    expect(places[1].realm).toBe(
      'https://worlds-content-server.decentraland.org/world/me.dcl.eth',
    );
  });

  it('tolerates garbage', () => {
    expect(parsePlaces(undefined)).toEqual([]);
    expect(parsePlaces({ data: 'x' })).toEqual([]);
  });
});

describe('parseEvents', () => {
  it('keeps live/trending flags and builds a detail place', () => {
    const [event] = parseEvents({
      data: [
        {
          id: 'e',
          name: 'Party',
          user_name: 'Ana',
          live: true,
          trending: true,
          total_attendees: 4,
          start_at: '2026-10-11T10:00:00.000Z',
          x: 5,
          y: -6,
          world: false,
        },
      ],
    });
    expect(event).toMatchObject({ live: true, trending: true, attendees: 4 });
    expect(event.place).toMatchObject({
      location: '5,-6',
      users: 4,
      likeRate: null,
      realm: mainRealm(),
      x: 5,
      y: -6,
    });
    expect(event.startsAt).toBe(Date.parse('2026-10-11T10:00:00.000Z'));
  });
});

describe('format', () => {
  const now = Date.parse('2026-10-11T12:00:00.000Z');
  it('describes when an event starts', () => {
    expect(startsIn(now + 5 * 60_000, now)).toBe('IN 5 MINS');
    expect(startsIn(now + 60_000, now)).toBe('IN 1 MIN');
    expect(startsIn(now + 2 * 3_600_000, now)).toBe('IN 2 HRS');
    expect(startsIn(now + 10_000, now)).toBe('STARTING');
    expect(startsIn(now + 3 * 86_400_000, now)).toBe('IN 3 DAYS');
    expect(startsIn(now + 9 * 86_400_000, now)).toMatch(/^[A-Z]{3} \d+$/);
  });

  it('formats rates, users and search', () => {
    expect(formatRate(0.876)).toBe('88%');
    expect(formatRate(null)).toBe('—');
    expect(formatUsers(7)).toBe('007');
    expect(matchesQuery('Genesis Plaza', ' plaza ')).toBe(true);
    expect(matchesQuery('Genesis Plaza', 'zzz')).toBe(false);
  });
});
