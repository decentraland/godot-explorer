import { setCoreForTests } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import { Control } from '../src/core/native';
import type { AppListener } from '../src/core/native';
import { coreHudApi, fallbackProfile } from '../src/hud/hudApi';

const profile = jest.fn();
const nearby = jest.fn();
let listener!: AppListener;

beforeEach(() => {
  profile.mockReset();
  nearby.mockReset();
  resetCoreEventsForTests();
  setCoreForTests({
    ...createMockCore(),
    profile,
    nearby,
    addListener: l => {
      listener = l;
      return 0n;
    },
  });
});

afterEach(() => setCoreForTests(undefined));

const person = (over = {}) => ({
  address: '0xabcdef1234',
  name: 'Neo',
  hasClaimedName: false,
  profilePictureUrl: '',
  ...over,
});

describe('coreHudApi events', () => {
  it('keys the profile the engine attaches to a chat line by its sender address', () => {
    const seen = jest.fn();
    coreHudApi.subscribe(seen);
    listener.chat({
      address: '0xabcdef1234',
      message: 'hi',
      timestamp: 1,
      profile: { name: 'Neo', hasClaimedName: false, profilePictureUrl: '' },
    });
    expect(seen.mock.calls[0][0].data.profile).toMatchObject({
      address: '0xabcdef1234',
      name: 'Neo',
      tag: '#1234',
    });
  });

  it('forwards a line without a profile as it is', () => {
    const seen = jest.fn();
    coreHudApi.subscribe(seen);
    listener.chat({
      address: 'system',
      message: 'welcome',
      timestamp: 2,
      profile: undefined,
    });
    expect(seen.mock.calls[0][0]).toEqual({
      type: 'chat',
      data: { address: 'system', message: 'welcome', timestamp: 2 },
    });
  });

  it('turns the loading detail into plain numbers', () => {
    const seen = jest.fn();
    coreHudApi.subscribe(seen);
    listener.loadingDetail({
      loaded: 412n,
      total: 584n,
      mbps: 3.2,
      percent: 71,
    });
    expect(seen).toHaveBeenCalledWith({
      type: 'loading_progress',
      data: { loaded: 412, total: 584, mbps: 3.2, percent: 71 },
    });
  });

  it('forwards the unread count and the backpack request', () => {
    const seen = jest.fn();
    coreHudApi.subscribe(seen);
    listener.unreadChat(3);
    listener.openBackpack(true);
    expect(seen.mock.calls.map(([event]) => event)).toEqual([
      { type: 'unread_chat', data: { count: 3 } },
      { type: 'open_backpack', data: { emotes: true } },
    ]);
  });

  it('stops forwarding after the unsubscribe', () => {
    const seen = jest.fn();
    coreHudApi.subscribe(seen)();
    listener.unreadChat(1);
    expect(seen).not.toHaveBeenCalled();
  });
});

describe('coreHudApi profiles', () => {
  it('resolves a sender through the core profile', async () => {
    profile.mockResolvedValue(
      person({ hasClaimedName: true, profilePictureUrl: 'https://x/face.png' }),
    );
    const resolved = await coreHudApi.resolveProfile('0xabcdef1234');
    expect(profile).toHaveBeenCalledWith('0xabcdef1234');
    expect(resolved).toMatchObject({
      name: 'Neo',
      tag: '',
      claimed: true,
      own: false,
      face: 'https://x/face.png',
    });
  });

  it('tags unclaimed names with the address suffix', async () => {
    profile.mockResolvedValue(person({ name: 'Trin' }));
    expect(await coreHudApi.resolveProfile('0xabcdef1234')).toMatchObject({
      name: 'Trin',
      tag: '#1234',
      claimed: false,
    });
  });

  it('falls back to the short address when the lookup fails', async () => {
    profile.mockRejectedValue(new Error('offline'));
    nearby.mockRejectedValue(new Error('offline'));
    expect(await coreHudApi.resolveProfile('0xabcdef1234')).toEqual(
      fallbackProfile('0xabcdef1234'),
    );
    expect((await coreHudApi.ownProfile()).own).toBe(true);
    expect(await coreHudApi.nearby()).toEqual([]);
  });

  it('reads the own profile without an address', async () => {
    profile.mockResolvedValue(
      person({ address: '0xme00', name: 'Me', hasClaimedName: true }),
    );
    expect(await coreHudApi.ownProfile()).toMatchObject({
      name: 'Me',
      own: true,
    });
    expect(profile).toHaveBeenCalledWith(undefined);
  });

  it('maps the nearby users to chat profiles', async () => {
    nearby.mockResolvedValue([person({ address: '0xaaaa0001', name: 'Cy' })]);
    expect(await coreHudApi.nearby()).toMatchObject([
      { address: '0xaaaa0001', name: 'Cy', tag: '#0001', claimed: false },
    ]);
  });

  it('sends commands to the core and enters worlds through it', () => {
    const control = jest.fn();
    const enterWorld = jest.fn();
    setCoreForTests({ ...createMockCore(), control, enterWorld });
    const teleport = new Control.Teleport({ parcel: { x: 1, y: 2 } });
    coreHudApi.send(teleport);
    expect(control).toHaveBeenCalledWith(teleport);
    coreHudApi.enterWorld({ realm: 'r', parcel: undefined });
    expect(enterWorld).toHaveBeenCalledWith({ realm: 'r', parcel: undefined });
  });
});
