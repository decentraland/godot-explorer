// Guards the translation catalogue: no hardcoded UI text in RN source, every key used exists in English,
// every English key is used, plural pairs are complete, and the untranslated count never grows.
// Escape hatch for text that must stay literal (brand names, dev-only): `// i18n-ignore` on the same or
// the previous line.
import fs from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import { en } from '../src/i18n/en';
import { engine } from '../src/i18n/en/engine';
import { table as es } from '../src/i18n/es';
import { table as ptBR } from '../src/i18n/pt_BR';

// Untranslated-key ceiling per locale; lower it as keys get translated.
const UNTRANSLATED_MAX = { es: 290, pt_BR: 290 };
// Keys in the catalogue with no caller yet; delete one here when it gets used or removed.
const UNUSED_ALLOWED = [
  'BACKPACK_BACKPACK',
  'DISCOVER_COORDINATES_PLACE',
  'DISCOVER_EXPAND',
  'LOADING_PERCENT',
  'MENU_FRIENDS_COUNT',
  'MENU_PLACE_EMPTY',
  'MENU_SHARE_EVENT',
  'MENU_SOCIAL_OFFLINE_NOW',
  'MODAL_BAN_PRE_CHECK_BODY',
  'MODAL_BAN_PRE_CHECK_PRIMARY',
  'MODAL_BAN_PRE_CHECK_TITLE',
];
const SRC = path.join(__dirname, '..', 'src');
// Props whose string value a player reads.
const TEXT_PROPS = new Set([
  'title',
  'subtitle',
  'label',
  'placeholder',
  'accessibilityLabel',
  'accessibilityHint',
  'text',
  'body',
  'message',
  'description',
  'cta',
]);
const SKIP = [
  `${path.sep}i18n${path.sep}`,
  `${path.sep}gallery${path.sep}`,
  `${path.sep}godot${path.sep}GodotPlaceholder`,
  'gallery.tsx',
  'mock',
  'simulator',
];

function sourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      sourceFiles(full, out);
    } else if (
      /\.(ts|tsx)$/.test(entry.name) &&
      !SKIP.some(s => full.includes(s))
    ) {
      out.push(full);
    }
  }
  return out;
}

const hasLetters = (s: string) => /[A-Za-zÀ-ɏ]{2,}/.test(s);

function hardcodedText(file: string): string[] {
  const text = fs.readFileSync(file, 'utf8');
  const sf = ts.createSourceFile(
    file,
    text,
    ts.ScriptTarget.Latest,
    true,
    file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const lines = text.split('\n');
  const found: string[] = [];
  const report = (node: ts.Node, value: string) => {
    const line = sf.getLineAndCharacterOfPosition(node.getStart()).line;
    if (
      /i18n-ignore/.test(lines[line] ?? '') ||
      /i18n-ignore/.test(lines[line - 1] ?? '')
    ) {
      return;
    }
    found.push(
      `${path.relative(SRC, file)}:${line + 1}  ${JSON.stringify(
        value.trim(),
      )}`,
    );
  };
  const visit = (node: ts.Node) => {
    if (ts.isJsxText(node) && hasLetters(node.text)) {
      report(node, node.text);
    } else if (
      ts.isJsxAttribute(node) &&
      TEXT_PROPS.has(node.name.getText()) &&
      node.initializer &&
      ts.isStringLiteral(node.initializer) &&
      hasLetters(node.initializer.text) &&
      // A catalogue key passed as a prop (`text="FTUE_TUTORIAL_EQUIP"`) is resolved by the callee.
      !(node.initializer.text in en)
    ) {
      report(node, node.initializer.text);
    } else if (
      ts.isJsxExpression(node) &&
      node.expression &&
      ts.isStringLiteralLike(node.expression) &&
      ts.isJsxElement(node.parent) &&
      hasLetters(node.expression.text)
    ) {
      report(node, node.expression.text);
    } else if (
      ts.isCallExpression(node) &&
      /^Alert\.alert$/.test(node.expression.getText())
    ) {
      node.arguments.slice(0, 2).forEach(a => {
        if (ts.isStringLiteralLike(a) && hasLetters(a.text)) {
          report(a, a.text);
        }
      });
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
  return found;
}

const files = sourceFiles(SRC);
const allSource = files.map(f => fs.readFileSync(f, 'utf8')).join('\n');
const keys = Object.keys(en as Record<string, string>);

describe('i18n coverage', () => {
  it('has no hardcoded user-facing text in RN source', () => {
    const found = files.flatMap(hardcodedText);
    expect(found).toEqual([]);
  });

  it('only calls t()/plural() with keys that exist in English', () => {
    const used = [
      ...allSource.matchAll(/\b(?:t|plural)\(\s*'([A-Z0-9_]+)'/g),
    ].map(m => m[1]);
    expect(used.filter(k => !(k in en))).toEqual([]);
  });

  it('uses every English key somewhere in the source', () => {
    // `t(`PREFIX_${x}`)` counts as a use of every key under PREFIX_.
    const prefixes = [...allSource.matchAll(/`([A-Z0-9_]+_)\$\{/g)].map(
      m => m[1],
    );
    const unused = keys.filter(
      k =>
        !k.endsWith('_PLURAL') &&
        // Engine-area keys are used by Rust and .tscn, not by RN source.
        !(k in engine) &&
        !allSource.includes(k) &&
        !prefixes.some(p => k.startsWith(p)) &&
        !UNUSED_ALLOWED.includes(k),
    );
    expect(unused).toEqual([]);
  });

  it('pairs every _PLURAL key with its singular', () => {
    const orphans = keys.filter(
      k => k.endsWith('_PLURAL') && !(k.replace(/_PLURAL$/, '') in en),
    );
    expect(orphans).toEqual([]);
  });

  it('keeps placeholders identical across locales', () => {
    const names = (s: string) =>
      [...s.matchAll(/\{(\w+)\}/g)]
        .map(m => m[1])
        .sort()
        .join(',');
    const english = en as Record<string, string>;
    const bad: string[] = [];
    for (const [locale, table] of [
      ['es', es],
      ['pt_BR', ptBR],
    ] as const) {
      for (const [k, v] of Object.entries(table)) {
        if (k in english && names(english[k]) !== names(v)) {
          bad.push(`${locale} ${k}`);
        }
      }
    }
    expect(bad).toEqual([]);
  });

  it('translates every engine key in every locale (Godot has no English fallback)', () => {
    const missing = Object.keys(engine).filter(k => !(k in es) || !(k in ptBR));
    expect(missing).toEqual([]);
  });

  // Ratchet: translate keys and lower these numbers; adding an English key without translations fails.
  it.each([
    ['es', es, UNTRANSLATED_MAX.es],
    ['pt_BR', ptBR, UNTRANSLATED_MAX.pt_BR],
  ] as const)('%s untranslated keys do not grow', (_locale, table, max) => {
    const missing = keys.filter(k => !(k in table));
    expect(missing.length).toBeLessThanOrEqual(max);
  });
});
