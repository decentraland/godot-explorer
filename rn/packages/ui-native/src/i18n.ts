/** The catalogue keys the components read. The host installs its translator; without one a key
 * renders as itself. Keys take no `{placeholder}` params: the first string that needs one widens
 * this contract, not just the catalogue. */
export type UiNativeKey =
  | 'APP_BACK'
  | 'APP_CREDITS'
  | 'APP_SEARCH'
  | 'APP_TICKET'
  | 'APP_CLEAR'
  | 'APP_CLEAR_SEARCH'
  | 'APP_COLOR_BRIGHTNESS'
  | 'APP_COLOR_HUE'
  | 'APP_COLOR_SATURATION'
  | 'APP_DISMISS_DIALOG';

let translate: (key: UiNativeKey) => string = key => key;

export function setTranslator(fn: (key: UiNativeKey) => string): void {
  translate = fn;
}

export const t = (key: UiNativeKey): string => translate(key);
