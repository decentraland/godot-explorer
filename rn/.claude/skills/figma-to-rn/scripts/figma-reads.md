# Read-only `use_figma` scripts for auditing a flow

Run with `use_figma` (create a session first). All read-only. File `H1Alvbe7sh0e0IyLckewMI`, page `HIFI`
= `1450:7148`. Re-run them every time: the designer edits frames in place and cleans the document.

## 1. The flow as drawn: frames in reading order + the notes + the decision groups
```js
const hifi = figma.root.children.find(p => p.name === 'HIFI'); await hifi.loadAsync();
const band = (lo, hi) => hifi.children.filter(c => c.y >= lo && c.y <= hi);
return {
  frames: band(-8200, -2400).filter(c => c.type === 'FRAME').sort((a, b) => a.x - b.x)
    .map(f => `${Math.round(f.x)},${Math.round(f.y)} ${f.name} ${f.id}`),
  notes: band(-8200, -1000).filter(c => c.type === 'TEXT').map(t => `${Math.round(t.x)},${Math.round(t.y)} ${t.id} ${JSON.stringify(t.characters)}`),
  decisions: band(-8200, -2000).filter(c => c.type === 'GROUP').map(g => g.name + ' ' + g.findAll(n => n.type === 'TEXT').map(t => t.characters).join('|')),
};
```
Rows repeat the same flow per provider (y 184 Google, y -4956 email, y 10898 returning user). Arrows are
`VECTOR`/`LINE` nodes, not connectors: their bounding boxes say who points to whom. TEXT notes carry behaviour.

## 2. Diff copies of a frame node by node (which copy is live, what changed)
```js
const spec = (n, ox, oy) => [n.type, Math.round(n.absoluteBoundingBox.x - ox), Math.round(n.absoluteBoundingBox.y - oy), Math.round(n.width * 10) / 10, Math.round(n.height * 10) / 10, n.visible === false ? 'hidden' : '', n.type === 'TEXT' ? JSON.stringify(n.characters) + ' ' + n.fontSize : ''].join(' ');
const flat = (n, ox, oy, path, o, d) => { const k = path + '/' + n.name; o[k] = spec(n, ox, oy); if (d > 0 && n.children) { const seen = {}; for (const c of n.children) { seen[c.name] = (seen[c.name] || 0) + 1; flat(c, ox, oy, k + (seen[c.name] > 1 ? '#' + seen[c.name] : ''), o, d - 1); } } };
const get = async id => { const f = await figma.getNodeByIdAsync(id); const o = {}; for (const c of f.children) flat(c, f.absoluteBoundingBox.x, f.absoluteBoundingBox.y, '', o, 12); return o; };
// compare get(oldId) with get(newId): keys whose value differs. Equal copies prove nothing about the CODE.
```

## 3. Text with mixed styles (legal lines, links, bold spans)
`getRangeFills(i,i+1)`, `getRangeTextDecoration`, `getRangeFontName` per character, grouped into runs.
One frame can underline links and another not (AUTH_HOME vs ACCOUNT_HOME): check each frame.

## 4. Component states (slots, fields, buttons)
`const mc = await instance.getMainComponentAsync(); mc.parent` is the COMPONENT_SET: list its children (State=Default/Typing/Filled/Error)
with fills, strokes and weights. `download_assets` does not accept ids inside instances: use the main component id.

## 5. Full-resolution frame for pixel comparison
`download_assets` with `defaultScale: 1, defaultFormat: png` returns a 720x1600 export (the screenshot tool is only 461 wide).
