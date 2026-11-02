import {
  backAction,
  initialNavState,
  navReducer,
  visibleRoutes,
} from '../src/app/navReducer';

const place = { id: 'p' } as never;

describe('navReducer', () => {
  it('pushes, pops and replaces', () => {
    let s = initialNavState();
    s = navReducer(s, { type: 'push', route: { name: 'profileEdit' } });
    s = navReducer(s, {
      type: 'push',
      route: { name: 'placeDetail', params: { placeId: 'a' } },
    });
    expect(s.stack.map(r => r.name)).toEqual([
      'home',
      'profileEdit',
      'placeDetail',
    ]);
    s = navReducer(s, {
      type: 'replace',
      route: { name: 'eventDetail', params: { eventId: 'e' } },
    });
    expect(s.stack[2].name).toBe('eventDetail');
    s = navReducer(s, { type: 'pop' });
    expect(s.stack).toHaveLength(2);
  });

  it('never pops the root and does not mutate', () => {
    const s = initialNavState();
    expect(navReducer(s, { type: 'pop' })).toBe(s);
    const pushed = navReducer(s, {
      type: 'push',
      route: { name: 'profileEdit' },
    });
    expect(s.stack).toHaveLength(1);
    expect(pushed).not.toBe(s);
  });

  it('reset replaces the whole stack and keeps the tab', () => {
    let s = navReducer(initialNavState(), { type: 'setTab', tab: 'settings' });
    s = navReducer(s, { type: 'push', route: { name: 'profileEdit' } });
    s = navReducer(s, { type: 'reset', route: { name: 'home' } });
    expect(s.stack).toEqual([{ name: 'home' }]);
    expect(s.tab).toBe('settings');
  });

  it('back pops, then returns to explore, then is unhandled', () => {
    let s = navReducer(initialNavState(), { type: 'setTab', tab: 'backpack' });
    s = navReducer(s, { type: 'push', route: { name: 'profileEdit' } });
    expect(backAction(s)).toEqual({ type: 'pop' });
    s = navReducer(s, { type: 'pop' });
    expect(backAction(s)).toEqual({ type: 'setTab', tab: 'explore' });
    s = navReducer(s, { type: 'setTab', tab: 'explore' });
    expect(backAction(s)).toBeNull();
  });

  it('back from the world is left to the leave confirmation', () => {
    const s = navReducer(initialNavState(), {
      type: 'push',
      route: { name: 'world', params: { place } },
    });
    expect(backAction(s)).toBeNull();
  });

  it('only the world and what is above it is visible', () => {
    let s = navReducer(initialNavState(), {
      type: 'push',
      route: { name: 'world', params: { place } },
    });
    s = navReducer(s, { type: 'push', route: { name: 'profileEdit' } });
    expect(visibleRoutes(s).map(r => r.name)).toEqual(['world', 'profileEdit']);
    expect(visibleRoutes(initialNavState()).map(r => r.name)).toEqual(['home']);
  });
});
