import { en } from './en';
import { table as es } from './es';
import { table as ptBR } from './pt_BR';

export type I18nKey = keyof typeof en;
export type I18nParams = Record<string, string | number>;

export type Locale = 'en' | 'es' | 'pt_BR';

const tables: Record<Locale, Record<string, string>> = {
  en: en as Record<string, string>,
  es,
  pt_BR: ptBR,
};

let current: Locale = 'en';
const listeners = new Set<() => void>();

export const getLocale = (): Locale => current;

/** Unknown values keep English; listeners (the root remount) run only when the locale changes. */
export function setLocale(next: string): void {
  const locale = next in tables ? (next as Locale) : 'en';
  if (locale !== current) {
    current = locale;
    listeners.forEach(l => l());
  }
}

export function subscribeLocale(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The text for `key` in the current locale, English where it has no translation. */
const lookup = (key: string): string | undefined =>
  tables[current][key] ?? (en as Record<string, string>)[key];

const fill = (text: string, params?: I18nParams): string =>
  params
    ? text.replace(/\{(\w+)\}/g, (whole, name: string) =>
        name in params ? String(params[name]) : whole,
      )
    : text;

/** Looks `key` up and fills named `{placeholders}`; a missing key renders as itself. */
export function t(key: I18nKey, params?: I18nParams): string {
  return fill(lookup(key) ?? key, params);
}

/** Picks `KEY` or `KEY_PLURAL` (n != 1; Brazilian Portuguese n > 1, like the CSV plural rules) and fills it. */
export function plural(
  key: I18nKey,
  count: number,
  params?: I18nParams,
): string {
  const form =
    (current === 'pt_BR' ? count > 1 : count !== 1) &&
    lookup(`${key}_PLURAL`) !== undefined
      ? `${key}_PLURAL`
      : key;
  return fill(lookup(form) ?? key, params);
}
