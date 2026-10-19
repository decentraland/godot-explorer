import {
  SLOW_AFTER_MS,
  TIPS,
  initialLoading,
  loadingReducer,
  percentLabel,
  progressLabel,
  progressValue,
  tipAt,
} from '../src/world/loadingModel';

const progress = { loaded: 3, total: 584, mbps: 0.1, percent: 10 };

describe('loadingReducer', () => {
  it('starts loading with an empty progress', () => {
    expect(initialLoading).toMatchObject({
      phase: 'loading',
      progress: { loaded: 0, total: 0, percent: 0 },
    });
  });

  it('stores progress while loading', () => {
    const s = loadingReducer(initialLoading, {
      type: 'progress',
      data: progress,
    });
    expect(s.progress).toEqual(progress);
  });

  it('finishes and stays finished', () => {
    const done = loadingReducer(initialLoading, { type: 'finished' });
    expect(done.phase).toBe('finished');
    expect(loadingReducer(done, { type: 'progress', data: progress })).toBe(
      done,
    );
    expect(loadingReducer(done, { type: 'slow' })).toBe(done);
  });

  it('goes slow only from loading, and run anyway finishes', () => {
    const slow = loadingReducer(initialLoading, { type: 'slow' });
    expect(slow.phase).toBe('slow');
    expect(loadingReducer(slow, { type: 'finished' }).phase).toBe('finished');
    expect(loadingReducer(slow, { type: 'dismiss' }).phase).toBe('finished');
    expect(loadingReducer(slow, { type: 'restart' })).toEqual(initialLoading);
  });

  it('records a failed load', () => {
    expect(loadingReducer(initialLoading, { type: 'failed' }).phase).toBe(
      'failed',
    );
  });

  it('90 seconds is the slow threshold', () => {
    expect(SLOW_AFTER_MS).toBe(90_000);
  });
});

describe('labels', () => {
  it('pads counts to two digits like the design', () => {
    expect(
      progressLabel({ loaded: 0, total: 0, mbps: 0.1, percent: 0 }),
    ).toEqual({
      loaded: '00',
      total: '00',
      speed: '0.10',
    });
    expect(progressLabel(progress)).toEqual({
      loaded: '03',
      total: '584',
      speed: '0.10',
    });
  });

  it('formats and clamps the percent', () => {
    expect(percentLabel(10.4)).toBe('10%');
    expect(percentLabel(140)).toBe('100%');
    expect(percentLabel(-3)).toBe('0%');
  });

  it('turns percent into a 0..1 bar value', () => {
    expect(progressValue(50)).toBe(0.5);
    expect(progressValue(250)).toBe(1);
  });
});

describe('tips', () => {
  it('covers the four icon variants and wraps around', () => {
    expect(new Set(TIPS.map(t => t.icon))).toEqual(
      new Set(['bulb', 'dancer', 'friends', 'chat']),
    );
    expect(tipAt(0)).toBe(TIPS[0]);
    expect(tipAt(TIPS.length)).toBe(TIPS[0]);
    expect(tipAt(-1)).toBe(TIPS[TIPS.length - 1]);
  });
});
