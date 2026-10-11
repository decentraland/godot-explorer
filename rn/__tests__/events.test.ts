import { initialHud, MAX_CHAT_LINES, parseEvent, reduceHud } from '../src/events';

const stateEvent = {
  type: 'state' as const,
  data: { parcel: [0, -2] as [number, number], realm: 'main', fps: 60, position: [1, 2, 3] as [number, number, number] },
};

describe('parseEvent', () => {
  it('parses a valid event', () => {
    expect(parseEvent(JSON.stringify(stateEvent))).toEqual(stateEvent);
  });

  it('rejects malformed json and shapes without type/data', () => {
    expect(parseEvent('not json')).toBeNull();
    expect(parseEvent('{"type":"state"}')).toBeNull();
    expect(parseEvent('42')).toBeNull();
  });
});

describe('reduceHud', () => {
  it('moves booting to loading on the first state event and keeps data', () => {
    const next = reduceHud(initialHud, stateEvent);
    expect(next.phase).toBe('loading');
    expect(next.parcel).toEqual([0, -2]);
    expect(next.fps).toBe(60);
  });

  it('only reaches playing after a state event and loading finished', () => {
    const early = reduceHud(initialHud, { type: 'loading', data: { state: 'finished' } });
    expect(early.phase).toBe('booting');
    const loading = reduceHud(initialHud, stateEvent);
    expect(reduceHud(loading, { type: 'loading', data: { state: 'finished' } }).phase).toBe('playing');
  });

  it('does not mutate the previous state and caps chat history', () => {
    let state = initialHud;
    for (let i = 0; i < MAX_CHAT_LINES + 5; i++) {
      state = reduceHud(state, { type: 'chat', data: { address: 'a', message: `m${i}`, timestamp: i } });
    }
    expect(state.chat).toHaveLength(MAX_CHAT_LINES);
    expect(state.chat[MAX_CHAT_LINES - 1].message).toBe(`m${MAX_CHAT_LINES + 4}`);
    expect(initialHud.chat).toHaveLength(0);
  });
});
