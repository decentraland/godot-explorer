import { en } from './en';

export type I18nKey = keyof typeof en;
export type I18nParams = Record<string, string | number>;

const fill = (text: string, params?: I18nParams): string =>
  params
    ? text.replace(/\{(\w+)\}/g, (whole, name: string) =>
        name in params ? String(params[name]) : whole,
      )
    : text;

/** Looks `key` up and fills named `{placeholders}`; a missing key renders as itself. */
export function t(key: I18nKey, params?: I18nParams): string {
  return fill((en as Record<string, string>)[key] ?? key, params);
}

/** Picks `KEY` or `KEY_PLURAL` (English rule: n != 1) and then fills it, like the Godot side. */
export function plural(
  key: I18nKey,
  count: number,
  params?: I18nParams,
): string {
  const table = en as Record<string, string>;
  const form = count !== 1 && `${key}_PLURAL` in table ? `${key}_PLURAL` : key;
  return fill(table[form] ?? key, params);
}
