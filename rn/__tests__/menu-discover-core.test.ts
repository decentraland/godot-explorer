import { createCoreMenuApi } from '../src/menu/coreMenuApi';
import { createMockApi as createMockMenuApi } from '../src/menu/mockApi';
import { forgetAll } from '../src/discover/catalog';

beforeEach(() => {
  forgetAll();
});

describe('menu core api', () => {
  it('serves the discover feed from the public places and events APIs', async () => {
    const place = {
      id: 'p1',
      title: 'Real Place',
      contact_name: 'Ana',
      image: 'https://img/p1.png',
      user_count: 7,
      like_rate: 0.8,
      base_position: '10,20',
    };
    const event = {
      id: 'e1',
      name: 'Real Event',
      user_name: 'Bo',
      image: 'https://img/e1.png',
      live: true,
      total_attendees: 3,
      start_at: '2026-10-12T12:00:00Z',
      x: 1,
      y: 2,
    };
    const fetchImpl = jest.fn(async (url: string) => ({
      ok: true,
      json: async () =>
        url.includes('/events') ? { data: [event] } : { data: [place] },
    })) as unknown as typeof fetch;
    const feed = await createCoreMenuApi(
      createMockMenuApi(),
      fetchImpl,
    ).discover();
    expect(feed.featured[0]).toMatchObject({
      title: 'Real Place',
      users: 7,
      image: 'https://img/p1.png',
    });
    expect(feed.events[0]).toMatchObject({ id: 'e1', attendees: 3 });
    expect(feed.featuredEvents).toEqual([]);
  });
});
