import { createCoreMenuApi } from '../src/menu/coreMenuApi';
import { createMockApi } from '../src/menu/mockApi';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { createMockCore } from '../src/core/mock';

describe('bug report submission', () => {
  afterEach(() => setCoreForTests(undefined));

  it('sends the form and the device facts to the core', async () => {
    const submitBugReport = jest.fn().mockResolvedValue('ticket');
    setCoreForTests({
      ...createMockCore(),
      submitBugReport,
      version: () => '2.0.0',
    } as CoreApi);
    const api = createCoreMenuApi(createMockApi());
    await api.submitBugReport({
      category: 'crash',
      description: 'froze',
      screenshots: ['file:///a.jpg'],
    });
    expect(submitBugReport).toHaveBeenCalledTimes(1);
    const sent = submitBugReport.mock.calls[0][0];
    expect(sent).toMatchObject({
      category: 'crash',
      description: 'froze',
      screenshots: ['file:///a.jpg'],
      clientVersion: '2.0.0',
      coordinates: '',
    });
    expect(typeof sent.ramMb).toBe('bigint');
  });

  it('lets the failure reach the form', async () => {
    setCoreForTests({
      ...createMockCore(),
      submitBugReport: jest.fn().mockRejectedValue(new Error('403')),
    } as CoreApi);
    const api = createCoreMenuApi(createMockApi());
    await expect(
      api.submitBugReport({
        category: 'chat',
        description: 'x',
        screenshots: [],
      }),
    ).rejects.toThrow('403');
  });
});
