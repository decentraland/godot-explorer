/// <reference types="node" />
import fs from 'fs';
import path from 'path';
import { en } from '../src/i18n/en';
import { plural, t } from '../src/i18n';

describe('t / plural', () => {
  it('looks up and fills named placeholders, repeats included', () => {
    expect(t('DISCOVER_DISCOVER')).toBe('Discover');
    expect(t('CALENDAR_TIME_RANGE', { start: 1, end: 'b', tz: 'UTC' })).toBe(
      '1 to b UTC',
    );
  });

  it('leaves unknown placeholders and returns the key when missing', () => {
    expect(t('CALENDAR_TIME_RANGE', { start: 1 })).toBe('1 to {end} {tz}');
    expect(t('NOPE' as never)).toBe('NOPE');
  });

  it('picks the plural form, then fills it', () => {
    expect(plural('EVENT_PILL_IN_DAY', 1, { days: 1 })).toBe('IN 1 DAY');
    expect(plural('EVENT_PILL_IN_DAY', 3, { days: 3 })).toBe('IN 3 DAYS');
    expect(plural('EVENT_PILL_IN_DAY', 0, { days: 0 })).toBe('IN 0 DAYS');
  });
});

const SRC = path.join(__dirname, '..', 'src');
const CSV = path.join(__dirname, '..', '..', 'godot', 'locale', 'en.csv');

function sources(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap(e => {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      return e.name === 'i18n' ? [] : sources(p);
    }
    return /\.tsx?$/.test(e.name) ? [p] : [];
  });
}

const names = (s: string) =>
  [...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort();

describe('key table', () => {
  it('every t()/plural() literal key exists', () => {
    const missing: string[] = [];
    for (const file of sources(SRC)) {
      const code = fs.readFileSync(file, 'utf8');
      for (const m of code.matchAll(/\b(?:t|plural)\(\s*'([A-Z0-9_]+)'/g)) {
        if (!(m[1] in en)) {
          missing.push(`${path.relative(SRC, file)}: ${m[1]}`);
        }
      }
    }
    expect(missing).toEqual([]);
  });

  it('every _PLURAL key has a singular and the same placeholders', () => {
    for (const key of Object.keys(en).filter(k => k.endsWith('_PLURAL'))) {
      const base = key.replace(/_PLURAL$/, '');
      expect(en).toHaveProperty(base);
      expect(names(en[key as keyof typeof en])).toEqual(
        names(en[base as keyof typeof en]),
      );
    }
  });

  it('keys shared with godot/locale keep the same placeholders', () => {
    const legacy = new Map<string, string>();
    for (const line of fs.readFileSync(CSV, 'utf8').split('\n')) {
      const m = line.match(/^"([A-Z0-9_]+)";"[^"]*";"[^"]*";"(.*)"$/);
      if (m) {
        legacy.set(m[1], m[2]);
      }
    }
    const drift = Object.entries(en).filter(
      ([k, v]) =>
        legacy.has(k) &&
        JSON.stringify(names(v)) !== JSON.stringify(names(legacy.get(k)!)),
    );
    expect(drift).toEqual([]);
  });
});
