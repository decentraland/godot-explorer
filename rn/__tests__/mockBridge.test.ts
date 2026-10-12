import { createMockBridge } from '../src/godot/mockBridge';
import { GodotEvent, initialHud, reduceHud } from '../src/events';

describe('createMockBridge', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it('replays a session that reaches the playing phase', () => {
    const bridge = createMockBridge();
    const events: GodotEvent[] = [];
    const unsubscribe = bridge.subscribe(event => events.push(event));
    jest.advanceTimersByTime(2000);
    unsubscribe();

    expect(events[0].type).toBe('ready');
    expect(events.some(event => event.type === 'state')).toBe(true);
    expect(events.reduce(reduceHud, initialHud).phase).toBe('playing');
  });

  it('echoes chat and moves on teleport', () => {
    const bridge = createMockBridge();
    const events: GodotEvent[] = [];
    const unsubscribe = bridge.subscribe(event => events.push(event));
    bridge.sendCommand({ type: 'send_chat', text: 'hi' });
    bridge.sendCommand({ type: 'teleport', x: 3, y: -2 });
    unsubscribe();

    expect(events).toContainEqual(
      expect.objectContaining({ type: 'chat', data: expect.objectContaining({ message: 'hi' }) }),
    );
    const state = events.filter(event => event.type === 'state').pop();
    expect(state?.type === 'state' && state.data.parcel).toEqual([3, -2]);
  });

  it('stops emitting once the last listener leaves', () => {
    const bridge = createMockBridge();
    const listener = jest.fn();
    bridge.subscribe(listener)();
    jest.advanceTimersByTime(2000);
    expect(listener).not.toHaveBeenCalled();
  });
});
