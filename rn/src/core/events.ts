import { core } from './index';
import type { AppListener, OpenUrl, State } from './native';

export type CoreHandlers = Partial<AppListener>;

const handlers = new Set<CoreHandlers>();
let registered = false;

/** The one native listener fans out to every subscriber, so screens never register their own. */
function ensureRegistered() {
  if (registered) {
    return;
  }
  registered = true;
  core().addListener({
    stateChanged: (state: State) => handlers.forEach(h => h.stateChanged?.(state)),
    sceneCrashed: () => handlers.forEach(h => h.sceneCrashed?.()),
    authError: (code: string) => handlers.forEach(h => h.authError?.(code)),
    openUrl: (request: OpenUrl) => handlers.forEach(h => h.openUrl?.(request)),
  });
}

/** Subscribes to the core's events; returns the unsubscribe function. */
export function onCoreEvents(subscriber: CoreHandlers): () => void {
  ensureRegistered();
  handlers.add(subscriber);
  return () => {
    handlers.delete(subscriber);
  };
}

export function resetCoreEventsForTests() {
  handlers.clear();
  registered = false;
}
