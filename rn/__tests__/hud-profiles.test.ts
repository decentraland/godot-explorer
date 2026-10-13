import { subscribe } from '../src/GodotBridge';
import { request } from '../src/bridge/request';
import { bridgeHudApi, fallbackProfile } from '../src/hud/hudApi';

jest.mock('../src/GodotBridge', () => ({
  sendCommand: jest.fn(),
  subscribe: jest.fn(() => () => {}),
}));
jest.mock('../src/bridge/request', () => ({ request: jest.fn() }));

const mockRequest = request as jest.Mock;

const mockSubscribe = subscribe as jest.Mock;

beforeEach(() => mockRequest.mockReset());

describe('bridgeHudApi chat events', () => {
  it('keys the profile Godot attaches to a chat line by its sender address', () => {
    let emit: (event: unknown) => void = () => {};
    mockSubscribe.mockImplementation(handler => {
      emit = handler;
      return () => {};
    });
    const listener = jest.fn();
    bridgeHudApi.subscribe(listener);
    emit({
      type: 'chat',
      data: {
        address: '0xabcdef1234',
        message: 'hi',
        timestamp: 1,
        profile: {
          name: 'Neo',
          has_claimed_name: false,
          profile_picture_url: '',
        },
      },
    });
    expect(listener.mock.calls[0][0].data.profile).toMatchObject({
      address: '0xabcdef1234',
      name: 'Neo',
      tag: '#1234',
    });
  });
});

describe('bridgeHudApi profiles', () => {
  it('resolves a sender through profile.get', async () => {
    mockRequest.mockResolvedValue({
      address: '0xabcdef1234',
      name: 'Neo',
      has_claimed_name: true,
      profile_picture_url: 'https://x/face.png',
    });
    const profile = await bridgeHudApi.resolveProfile('0xabcdef1234');
    expect(mockRequest).toHaveBeenCalledWith('profile.get', {
      address: '0xabcdef1234',
    });
    expect(profile).toMatchObject({
      name: 'Neo',
      tag: '',
      claimed: true,
      own: false,
      face: 'https://x/face.png',
    });
  });

  it('tags unclaimed names with the address suffix', async () => {
    mockRequest.mockResolvedValue({ address: '0xabcdef1234', name: 'Trin' });
    expect(await bridgeHudApi.resolveProfile('0xabcdef1234')).toMatchObject({
      name: 'Trin',
      tag: '#1234',
      claimed: false,
    });
  });

  it('falls back to the short address when the lookup fails', async () => {
    mockRequest.mockRejectedValue(new Error('no mock handler'));
    expect(await bridgeHudApi.resolveProfile('0xabcdef1234')).toEqual(
      fallbackProfile('0xabcdef1234'),
    );
    expect((await bridgeHudApi.ownProfile()).own).toBe(true);
    expect(await bridgeHudApi.nearby()).toEqual([]);
  });

  it('reads the own profile without an address', async () => {
    mockRequest.mockResolvedValue({
      address: '0xme00',
      name: 'Me',
      has_claimed_name: true,
    });
    expect(await bridgeHudApi.ownProfile()).toMatchObject({
      name: 'Me',
      own: true,
    });
    expect(mockRequest).toHaveBeenCalledWith('profile.get', undefined);
  });

  it('maps friends.nearby to chat profiles', async () => {
    mockRequest.mockResolvedValue({
      users: [
        {
          address: '0xaaaa0001',
          name: 'Cy',
          has_claimed_name: false,
          profile_picture_url: '',
        },
      ],
    });
    expect(await bridgeHudApi.nearby()).toMatchObject([
      { address: '0xaaaa0001', name: 'Cy', tag: '#0001', claimed: false },
    ]);
  });
});
