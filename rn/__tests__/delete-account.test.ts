import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { AccountDeletion } from '../src/core/native';
import { createCoreSettingsApi } from '../src/settings/coreApi';

afterEach(() => setCoreForTests(undefined));

async function deleteWith(result: AccountDeletion | Error) {
  const core = createMockCore();
  core.deleteAccount = () =>
    result instanceof Error ? Promise.reject(result) : Promise.resolve(result);
  setCoreForTests(core);
  return createCoreSettingsApi().deleteAccount();
}

describe('account deletion in settings', () => {
  it('reports a guest removed from the device as deleted', async () => {
    expect(await deleteWith(AccountDeletion.Deleted)).toEqual({
      ok: true,
      deleted: true,
    });
  });

  it('reports a server request as requested, not deleted', async () => {
    expect(await deleteWith(AccountDeletion.Requested)).toEqual({
      ok: true,
      deleted: false,
    });
  });

  it('carries the failure message', async () => {
    expect(await deleteWith(new Error('offline'))).toEqual({
      ok: false,
      error: 'offline',
    });
  });
});
