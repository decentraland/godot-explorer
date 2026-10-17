import type { ModalApi } from './ModalProvider';

let current: ModalApi | null = null;

/** The provider registers itself so code outside React (a menu action) can raise a modal. */
export function registerModalApi(api: ModalApi | null): void {
  current = api;
}

export function modalApi(): ModalApi | null {
  return current;
}
