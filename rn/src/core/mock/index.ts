import type { CoreApi } from '../api';
import { createMockSession } from './session';

/** Implements the same interface as the native binding, for builds without the engine library. */
export function createMockCore(): CoreApi {
  return { ...createMockSession() };
}
