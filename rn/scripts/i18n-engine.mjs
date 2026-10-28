#!/usr/bin/env node
// Exports the `engine` area of the RN catalogue (src/i18n/<locale>/engine.ts) to godot/locale/<locale>.csv,
// the only translations the Godot engine loads. The RN catalogue is the single source of truth.
//
//   node scripts/i18n-engine.mjs           write the CSVs
//   node scripts/i18n-engine.mjs --check   fail if the CSVs are stale or a locale lacks an engine key
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const i18n = path.join(root, 'src', 'i18n');
const csvDir = path.join(root, '..', 'godot', 'locale');
const check = process.argv.includes('--check');
const PLURAL_RULE = { en: 'n != 1', es: 'n != 1', pt_BR: 'n > 1' };

const table = locale => {
  const source = fs.readFileSync(path.join(i18n, locale, 'engine.ts'), 'utf8');
  const out = {};
  for (const m of source.matchAll(
    /^ {2}([A-Z0-9_]+):\s*'((?:[^'\\]|\\.)*)'/gm,
  )) {
    out[m[1]] = m[2].replace(/\\'/g, "'");
  }
  return out;
};

const quote = s => `"${s.replace(/"/g, '""')}"`;
const english = table('en');
const problems = [];
const files = [];
for (const locale of Object.keys(PLURAL_RULE)) {
  const text = table(locale);
  const lines = [
    ['keys', '?plural', '?context', locale],
    ['?pluralrule', '', '', `nplurals=2; plural=(${PLURAL_RULE[locale]});`],
  ];
  for (const key of Object.keys(english)) {
    if (!text[key]) {
      problems.push(
        `${locale}: engine key ${key} is not translated (Godot has no English fallback here)`,
      );
      continue;
    }
    lines.push([key, '', '', text[key]]);
  }
  files.push([
    path.join(csvDir, `${locale}.csv`),
    lines.map(r => r.map(quote).join(';')).join('\n') + '\n',
  ]);
}

for (const [file, content] of files) {
  if (check) {
    if (!fs.existsSync(file) || fs.readFileSync(file, 'utf8') !== content) {
      problems.push(
        `${path.relative(
          root,
          file,
        )} is stale (run node scripts/i18n-engine.mjs)`,
      );
    }
  } else {
    fs.writeFileSync(file, content);
  }
}
if (problems.length) {
  console.error(problems.join('\n'));
  process.exit(1);
}
