// On-device check of the core binding (3.1): the module loads, an async call returns a promise,
// and a JS listener receives events fired from a tokio thread. Logged as [CoreSelfTest] and run
// only in dev builds, where the native module exists (the no-engine builds have none).
type CoreModule = typeof import('dcl-core');

const EVENTS = 1000;
const EVENTS_PER_SECOND = 1000;

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) => setTimeout(() => reject(new Error('timeout')), ms)),
  ]);
}

export async function runCoreSelfTest(): Promise<void> {
  const log = (...args: unknown[]) => console.log('[CoreSelfTest]', ...args);
  let core: CoreModule;
  try {
    core = require('dcl-core');
  } catch {
    log('no native core in this build');
    return;
  }

  try {
    log('installs', core.installs(), 'state', core.state().tag);

    const started = Date.now();
    const slept = await core.debugDelay(200);
    log('async call resolved', slept, 'after', Date.now() - started, 'ms');

    for (const count of [0, 1, 10, 100, EVENTS]) {
      let received = 0;
      const listener: import('dcl-core').AppListener = {
        stateChanged: () => {
          received += 1;
        },
        sceneCrashed: () => {},
        authError: () => {},
        openUrl: () => {},
      };
      const id = core.addListener(listener);
      const fired = await withTimeout(
        core.debugFireEvents(count, EVENTS_PER_SECOND),
        10000,
      );
      core.removeListener(id);
      await new Promise<void>(resolve => setTimeout(resolve, 500));
      log('events', received, 'of', fired, received === fired ? 'PASS' : 'FAIL');
    }
  } catch (e) {
    log('FAIL', String(e), JSON.stringify((e as {inner?: unknown}).inner));
  }
}
