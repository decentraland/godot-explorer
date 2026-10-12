import { useSyncExternalStore } from 'react';
import { AvatarState, GodotEvent } from './events';
import {
  GodotCommand,
  sendCommand,
  startEngine,
  subscribe,
} from './GodotBridge';

/** Godot engine lifecycle as RN sees it: lazy start, command queue until `ready`, latest avatar. */
export type EngineSnapshot = {
  started: boolean;
  ready: boolean;
  avatar: AvatarState | null;
};

let snapshot: EngineSnapshot = { started: false, ready: false, avatar: null };
let queue: GodotCommand[] = [];
let unsubscribe: (() => void) | null = null;
const listeners = new Set<() => void>();

function publish(next: EngineSnapshot) {
  snapshot = next;
  listeners.forEach(listener => listener());
}

/** Placement commands supersede each other; everything else must reach Godot in order. */
function enqueue(command: GodotCommand) {
  if (command.type === 'backpack_open' || command.type === 'backpack_place') {
    queue = queue.filter(
      queued =>
        queued.type !== 'backpack_open' && queued.type !== 'backpack_place',
    );
    queue.push({ ...command, type: 'backpack_open' });
    return;
  }
  queue.push(command);
}

function handleEvent(event: GodotEvent) {
  if (event.type === 'ready') {
    const pending = queue;
    queue = [];
    publish({ ...snapshot, ready: true });
    pending.forEach(sendCommand);
  } else if (event.type === 'avatar') {
    publish({ ...snapshot, avatar: event.data });
  }
}

/** Starts listening to Godot and boots the engine once; later calls are no-ops. */
export function ensureEngine() {
  if (!unsubscribe) {
    unsubscribe = subscribe(handleEvent);
  }
  if (!snapshot.started) {
    publish({ ...snapshot, started: true });
    startEngine();
  }
}

export function send(command: GodotCommand) {
  if (snapshot.ready) {
    sendCommand(command);
  } else {
    enqueue(command);
  }
}

export function getEngine(): EngineSnapshot {
  return snapshot;
}

export function useEngine(): EngineSnapshot {
  return useSyncExternalStore(listener => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  }, getEngine);
}

/** Test hook: forget everything, as after a fresh process start. */
export function resetEngineForTests() {
  unsubscribe?.();
  unsubscribe = null;
  queue = [];
  snapshot = { started: false, ready: false, avatar: null };
  listeners.clear();
}
