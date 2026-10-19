import { State } from '../native';
import type { AppListener } from '../native';

/**
 * The app state and the listeners every mock slice shares, so a slice that changes the state
 * (entering a world) reaches the ones who subscribed through `addListener`.
 */
export type MockBus = {
  state(): State;
  setState(next: State): void;
  add(listener: AppListener): bigint;
  remove(id: bigint): void;
  /** Calls `notify` on every listener. */
  emit(notify: (listener: AppListener) => void): void;
};

export function createMockBus(initial: State = new State.SignedIn()): MockBus {
  let state = initial;
  let nextId = 0n;
  const listeners = new Map<bigint, AppListener>();
  const emit = (notify: (listener: AppListener) => void) =>
    listeners.forEach(notify);
  return {
    state: () => state,
    setState: next => {
      state = next;
      emit(l => l.stateChanged(next));
    },
    add: listener => {
      listeners.set(nextId, listener);
      return nextId++;
    },
    remove: id => {
      listeners.delete(id);
    },
    emit,
  };
}

/** The bus `createMockCore` hands to the control and scene mocks. */
export const mockBus: MockBus = createMockBus();
