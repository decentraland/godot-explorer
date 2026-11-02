import { AppState } from 'react-native';
import { act, create } from 'react-test-renderer';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { TICK_MS, useSessionTracking } from '../src/app/useSessionTracking';

function Probe() {
  useSessionTracking();
  return null;
}

describe('session tracking', () => {
  afterEach(() => {
    jest.useRealTimers();
    jest.restoreAllMocks();
    setCoreForTests(undefined);
  });

  it('reports launch, a heartbeat and what the app does', () => {
    jest.useFakeTimers();
    const calls: string[] = [];
    let listener: (state: string) => void = () => {};
    jest.spyOn(AppState, 'addEventListener').mockImplementation(((
      _: string,
      handler: (state: string) => void,
    ) => {
      listener = handler;
      return { remove: () => {} };
    }) as never);
    setCoreForTests({
      ...createMockCore(),
      sessionLaunched: () => calls.push('launched'),
      sessionTick: () => calls.push('tick'),
      sessionForegrounded: () => calls.push('foreground'),
      sessionBackgrounded: () => calls.push('background'),
    } as CoreApi);

    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(<Probe />);
    });
    expect(calls).toEqual(['launched']);
    act(() => {
      jest.advanceTimersByTime(TICK_MS * 2);
    });
    expect(calls.filter(c => c === 'tick')).toHaveLength(2);
    act(() => listener('background'));
    act(() => listener('active'));
    expect(calls.slice(-2)).toEqual(['background', 'foreground']);
    act(() => tree.unmount());
    act(() => {
      jest.advanceTimersByTime(TICK_MS * 3);
    });
    expect(calls.filter(c => c === 'tick')).toHaveLength(2);
  });

  it('survives a core without analytics', () => {
    setCoreForTests({
      ...createMockCore(),
      sessionLaunched: () => {
        throw new Error('analytics is not installed');
      },
    } as CoreApi);
    expect(() =>
      act(() => {
        create(<Probe />);
      }),
    ).not.toThrow();
  });
});
