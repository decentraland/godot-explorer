import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import type { AppListener } from '../src/core/native';

const mockBrowser = {
  isAvailable: jest.fn(),
  open: jest.fn(),
  close: jest.fn(),
};
jest.mock('react-native-inappbrowser-reborn', () => ({
  __esModule: true,
  default: mockBrowser,
}));

afterEach(() => setCoreForTests(undefined));

async function openedBy(result: { type: 'cancel' | 'dismiss' }) {
  mockBrowser.isAvailable.mockResolvedValue(true);
  mockBrowser.open.mockResolvedValue(result);
  const core = createMockCore();
  let listener: Partial<AppListener> | undefined;
  const add = core.addListener.bind(core);
  core.addListener = l => {
    listener = l;
    return add(l);
  };
  setCoreForTests(core);
  const { startLinkHandling } = require('../src/flow/browser');
  startLinkHandling();
  core.signInBrowser('google');
  listener?.openUrl?.({ url: 'https://auth.example/login' } as never);
  await new Promise(resolve => setImmediate(resolve));
  return core;
}

describe('the sign-in browser', () => {
  it('ends the sign-in when the player closes the tab', async () => {
    const core = await openedBy({ type: 'cancel' });
    expect(mockBrowser.open).toHaveBeenCalledWith('https://auth.example/login');
    expect(core.state().tag).toBe('SignedOut');
  });

  it('keeps the sign-in going when the tab is dismissed after the link returned', async () => {
    const core = await openedBy({ type: 'dismiss' });
    expect(core.state().tag).toBe('SigningIn');
  });
});
