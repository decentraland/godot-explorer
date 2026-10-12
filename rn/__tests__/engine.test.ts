import * as bridge from '../src/GodotBridge';
import {
  ensureEngine,
  getEngine,
  resetEngineForTests,
  send,
} from '../src/engine';

jest.mock('../src/GodotBridge', () => ({
  sendCommand: jest.fn(),
  startEngine: jest.fn(),
  setOrientation: jest.fn(),
  subscribe: jest.fn(),
}));

const mocked = bridge as jest.Mocked<typeof bridge>;
let emit: (event: unknown) => void;

beforeEach(() => {
  jest.clearAllMocks();
  resetEngineForTests();
  mocked.subscribe.mockImplementation(listener => {
    emit = listener as (event: unknown) => void;
    return () => {};
  });
});

describe('engine', () => {
  it('boots the engine once', () => {
    ensureEngine();
    ensureEngine();
    expect(mocked.startEngine).toHaveBeenCalledTimes(1);
    expect(getEngine().started).toBe(true);
  });

  it('queues commands until ready, collapsing placement, then flushes in order', () => {
    ensureEngine();
    send({ type: 'backpack_open', x: 0, y: 0, w: 10, h: 10 });
    send({ type: 'equip', urn: 'a' });
    send({ type: 'backpack_place', x: 1, y: 2, w: 30, h: 40 });
    expect(mocked.sendCommand).not.toHaveBeenCalled();
    emit({ type: 'ready', data: {} });
    expect(mocked.sendCommand.mock.calls.map(call => call[0])).toEqual([
      { type: 'equip', urn: 'a' },
      { type: 'backpack_open', x: 1, y: 2, w: 30, h: 40 },
    ]);
    send({ type: 'unequip', urn: 'a' });
    expect(mocked.sendCommand).toHaveBeenCalledTimes(3);
  });

  it('keeps the latest avatar state', () => {
    ensureEngine();
    const avatar = {
      body_shape: 'm',
      wearables: [],
      emotes: [],
      skin: '#000000',
      hair: '#000000',
      eyes: '#000000',
    };
    emit({ type: 'avatar', data: avatar });
    expect(getEngine().avatar).toEqual(avatar);
  });

  it('tracks the world and resets the hud when leaving it', () => {
    ensureEngine();
    emit({ type: 'world', data: { state: 'entered' } });
    emit({
      type: 'state',
      data: { parcel: [1, 2], realm: 'main', fps: 60, position: [0, 0, 0] },
    });
    expect(getEngine().inWorld).toBe(true);
    expect(getEngine().hud.parcel).toEqual([1, 2]);
    emit({ type: 'world', data: { state: 'exited' } });
    expect(getEngine().inWorld).toBe(false);
    expect(getEngine().hud.parcel).toBeNull();
  });
});
