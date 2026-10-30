#!/usr/bin/env node
// Screenshots gallery screens from the react-native-web export of the app, one PNG per id.
//   node tools/ui-spec/web_capture.mjs --dir <export dir> --out <dir> (--set | --ids a,b | --source s1,s2)
// 360x800 dp at 2x (the Figma frame size), or 800x360 for a landscape screen. CHANNEL=chrome uses the installed Chrome.
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { mkdirSync, readFileSync, existsSync, statSync, createReadStream } from 'node:fs';
import { dirname, extname, join, normalize, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const { chromium } = createRequire(join(ROOT, 'rn', 'package.json'))('playwright-core');
const GALLERY = join(ROOT, 'rn', 'src', 'gallery');
const READY_TIMEOUT_MS = 20000;
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.ttf': 'font/ttf', '.png': 'image/png', '.json': 'application/json' };

const args = new Map();
const argv = process.argv.slice(2);
for (let i = 0; i < argv.length; i++) {
  if (argv[i].startsWith('--')) {
    const next = argv[i + 1];
    const hasValue = next !== undefined && !next.startsWith('--');
    args.set(argv[i].slice(2), hasValue ? next : true);
    if (hasValue) i++;
  }
}
const dir = resolve(String(args.get('dir') ?? ''));
const out = resolve(String(args.get('out') ?? ''));
if (!args.has('dir') || !args.has('out') || !existsSync(dir)) {
  console.error('usage: web_capture.mjs --dir <export dir> --out <dir> (--set | --ids a,b | --source s1,s2)');
  process.exit(2);
}

let rows = JSON.parse(readFileSync(join(GALLERY, 'ids.json'), 'utf8'));
if (args.has('set')) {
  const wanted = new Set(JSON.parse(readFileSync(join(GALLERY, 'snapshot-set.json'), 'utf8')));
  rows = rows.filter(r => wanted.has(r.id));
} else if (args.has('ids')) {
  const wanted = new Set(String(args.get('ids')).split(','));
  rows = rows.filter(r => wanted.has(r.id));
} else if (args.has('source')) {
  const wanted = new Set(String(args.get('source')).split(','));
  rows = rows.filter(r => wanted.has(r.source));
} else {
  console.error('one of --set, --ids, --source is required');
  process.exit(2);
}

const server = createServer((req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname));
  let file = join(dir, path);
  if (!file.startsWith(dir) || !existsSync(file) || statSync(file).isDirectory()) {
    file = join(dir, 'index.html');
  }
  res.setHeader('content-type', TYPES[extname(file)] ?? 'application/octet-stream');
  createReadStream(file).pipe(res);
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const base = `http://127.0.0.1:${server.address().port}`;

const browser = await chromium.launch(process.env.CHANNEL ? { channel: process.env.CHANNEL } : {});
const failed = [];
for (const { id, orientation } of rows) {
  const landscape = orientation === 'landscape';
  const context = await browser.newContext({
    viewport: landscape ? { width: 800, height: 360 } : { width: 360, height: 800 },
    deviceScaleFactor: 2,
    reducedMotion: 'reduce',
  });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', e => problems.push(e.stack ?? e.message));
  try {
    await page.goto(`${base}/?preview=${encodeURIComponent(id)}`);
    await page.waitForSelector('[data-testid="preview-ready"]', { state: 'attached', timeout: READY_TIMEOUT_MS });
    await page.evaluate(() => document.fonts.ready);
    const file = join(out, `${id}.png`);
    mkdirSync(dirname(file), { recursive: true });
    await page.screenshot({ path: file, animations: 'disabled', caret: 'hide' });
    console.log(`ok ${id}`);
  } catch (error) {
    failed.push(id);
    console.error(`FAILED ${id}: ${String(error.message).split('\n')[0]}`);
    problems.slice(0, 3).forEach(p => console.error(`  ${p.split('\n').slice(0, 4).join('\n  ')}`));
  } finally {
    await context.close();
  }
}
await browser.close();
server.close();
if (failed.length) {
  console.error(`${failed.length} screen(s) not captured`);
  process.exit(1);
}
