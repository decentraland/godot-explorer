import type { GodotCommand } from '../src/GodotBridge';
import type { GodotEvent } from '../src/events';
import { createMockBridge } from '../src/godot/mockBridge';
import {
  clearMockHandlers,
  registerMockHandler,
} from '../src/bridge/mockHandlers';
import { BridgeError, request, resetRequests } from '../src/bridge/request';

// Route GodotBridge through a controllable fake so the request layer is tested in isolation.
const mockSent: GodotCommand[] = [];
let mockListeners: Array<(event: GodotEvent) => void> = [];
let mockUseMock = false;
let mockBridge = createMockBridge();

jest.mock('../src/GodotBridge', () => ({
  sendCommand: (command: GodotCommand) => {
    mockSent.push(command);
    if (mockUseMock) {
      mockBridge.sendCommand(command);
    }
  },
  subscribe: (listener: (event: GodotEvent) => void) => {
    mockListeners.push(listener);
    return () => {
      mockListeners = mockListeners.filter(item => item !== listener);
    };
  },
}));

const deliver = (event: GodotEvent) => mockListeners.forEach(l => l(event));

beforeEach(() => {
  mockSent.length = 0;
  mockUseMock = false;
  clearMockHandlers();
  resetRequests();
  jest.useRealTimers();
});

describe('request', () => {
  it('sends a req command and resolves with the matching res result', async () => {
    const promise = request('settings.version');
    const cmd = mockSent[0] as Extract<GodotCommand, { type: 'req' }>;
    expect(cmd.type).toBe('req');
    expect(cmd.method).toBe('settings.version');
    deliver({
      type: 'res',
      data: { id: cmd.id, ok: true, result: { version: '1.2.3' } },
    });
    await expect(promise).resolves.toEqual({ version: '1.2.3' });
  });

  it('uses a unique id per call and resolves each independently', async () => {
    const a = request('settings.version');
    const b = request('settings.cache_size');
    const [ca, cb] = mockSent as Array<Extract<GodotCommand, { type: 'req' }>>;
    expect(ca.id).not.toBe(cb.id);
    deliver({
      type: 'res',
      data: { id: cb.id, ok: true, result: { bytes: 1, max_bytes: 2 } },
    });
    deliver({
      type: 'res',
      data: { id: ca.id, ok: true, result: { version: 'x' } },
    });
    await expect(a).resolves.toEqual({ version: 'x' });
    await expect(b).resolves.toEqual({ bytes: 1, max_bytes: 2 });
  });

  it('rejects with a BridgeError when the response is not ok', async () => {
    const promise = request('account.sign_out');
    const cmd = mockSent[0] as Extract<GodotCommand, { type: 'req' }>;
    deliver({ type: 'res', data: { id: cmd.id, ok: false, error: 'nope' } });
    await expect(promise).rejects.toBeInstanceOf(BridgeError);
    await expect(promise).rejects.toThrow('nope');
  });

  it('rejects after the timeout and ignores a late response', async () => {
    jest.useFakeTimers();
    const promise = request('settings.version', undefined, { timeoutMs: 50 });
    const cmd = mockSent[0] as Extract<GodotCommand, { type: 'req' }>;
    const assertion = expect(promise).rejects.toThrow(/timed out/);
    jest.advanceTimersByTime(60);
    await assertion;
    expect(() =>
      deliver({ type: 'res', data: { id: cmd.id, ok: true, result: {} } }),
    ).not.toThrow();
  });

  it('ignores responses with unknown ids and unrelated events', async () => {
    const promise = request('settings.version');
    const cmd = mockSent[0] as Extract<GodotCommand, { type: 'req' }>;
    deliver({ type: 'res', data: { id: 'other', ok: true, result: 1 } });
    deliver({ type: 'ready', data: {} });
    deliver({
      type: 'res',
      data: { id: cmd.id, ok: true, result: { version: 'ok' } },
    });
    await expect(promise).resolves.toEqual({ version: 'ok' });
  });

  it('passes params through', () => {
    request('settings.set', { key: 'volume', value: 3, commit: true }).catch(
      () => {},
    );
    expect((mockSent[0] as { params: unknown }).params).toEqual({
      key: 'volume',
      value: 3,
      commit: true,
    });
  });
});

describe('mock handler registry', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it('answers requests through the mock bridge', async () => {
    mockUseMock = true;
    mockBridge = createMockBridge();
    mockBridge.subscribe(listener => deliver(listener));
    registerMockHandler('settings.version', () => ({ version: 'mock-1' }));
    await expect(request('settings.version')).resolves.toEqual({
      version: 'mock-1',
    });
  });

  it('turns handler exceptions and missing handlers into rejections', async () => {
    mockUseMock = true;
    mockBridge = createMockBridge();
    mockBridge.subscribe(listener => deliver(listener));
    registerMockHandler('account.sign_out', () => {
      throw new Error('boom');
    });
    await expect(request('account.sign_out')).rejects.toThrow('boom');
    await expect(request('account.delete')).rejects.toThrow(/no mock handler/);
  });

  it('supports async handlers and receives params', async () => {
    mockUseMock = true;
    mockBridge = createMockBridge();
    mockBridge.subscribe(listener => deliver(listener));
    registerMockHandler('clipboard.copy', async (params: { text: string }) => ({
      copied: params.text.length,
    }));
    await expect(request('clipboard.copy', { text: 'hello' })).resolves.toEqual(
      { copied: 5 },
    );
  });
});
