// Fake method responses for the mock bridge: area mock APIs and tests register what a method returns.
type Handler = (params: never) => unknown | Promise<unknown>;

const handlers = new Map<string, Handler>();

export function registerMockHandler<P = never>(
  method: string,
  handler: (params: P) => unknown | Promise<unknown>,
): void {
  handlers.set(method, handler as Handler);
}

export function clearMockHandlers(): void {
  handlers.clear();
}

export function getMockHandler(method: string): Handler | undefined {
  return handlers.get(method);
}
