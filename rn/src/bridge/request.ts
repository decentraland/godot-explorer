// Promise-based request/response over the command/event channel:
// `{type:'req', id, method, params}` out, `{type:'res', data:{id, ok, result|error}}` back.
import { sendCommand, subscribe } from '../GodotBridge';
import type { MethodName, MethodParams, MethodResult } from './methods';

export const DEFAULT_TIMEOUT_MS = 15000;

export class BridgeError extends Error {
  constructor(message: string, readonly method: string) {
    super(message);
    this.name = 'BridgeError';
  }
}

type Pending = {
  method: string;
  resolve: (value: never) => void;
  reject: (error: BridgeError) => void;
  timer: ReturnType<typeof setTimeout>;
};

const pending = new Map<string, Pending>();
let unsubscribe: (() => void) | null = null;
let counter = 0;

function ensureSubscribed(): void {
  if (unsubscribe) {
    return;
  }
  unsubscribe = subscribe(event => {
    if (event.type !== 'res') {
      return;
    }
    const entry = pending.get(event.data.id);
    if (!entry) {
      return;
    }
    pending.delete(event.data.id);
    clearTimeout(entry.timer);
    if (event.data.ok) {
      entry.resolve(event.data.result as never);
    } else {
      entry.reject(
        new BridgeError(event.data.error ?? 'request failed', entry.method),
      );
    }
  });
}

export function request<M extends MethodName>(
  method: M,
  params?: MethodParams<M>,
  options: { timeoutMs?: number } = {},
): Promise<MethodResult<M>> {
  ensureSubscribed();
  counter += 1;
  const id = `r${counter}`;
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  return new Promise<MethodResult<M>>((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(
        new BridgeError(`${method} timed out after ${timeoutMs}ms`, method),
      );
    }, timeoutMs);
    pending.set(id, { method, resolve, reject, timer });
    sendCommand({ type: 'req', id, method, params });
  });
}

/** Test helper: drops in-flight requests and the event subscription. */
export function resetRequests(): void {
  pending.forEach(entry => clearTimeout(entry.timer));
  pending.clear();
  unsubscribe?.();
  unsubscribe = null;
  counter = 0;
}
