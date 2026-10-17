// Check the actual release artifact, including its JS/Wasm pairing and MIME header.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const root = new URL('../dist/web/', import.meta.url);
const html = await readFile(new URL('index.html', root), 'utf8');
const path = html.match(/import\('\.\/(assets\/([a-f0-9]{24})\/math_playground\.js)'\)/);
assert.ok(path, 'HTML must reference the versioned JS/Wasm directory');
const js = await readFile(new URL(path[1], root));
const wasm = await readFile(new URL(path[1].replace(/\.js$/, '_bg.wasm'), root));
assert.equal(createHash('sha256').update(js).update(wasm).digest('hex').slice(0, 24), path[2]);
assert.ok(js.toString().includes("new URL('math_playground_bg.wasm', import.meta.url)"), 'Wasm must resolve beside its JS');
assert.deepEqual([...wasm.subarray(0, 8)], [0, 97, 115, 109, 1, 0, 0, 0]);
assert.ok(wasm.length < 25 * 1024 * 1024);
const headers = await readFile(new URL('_headers', root), 'utf8');
assert.match(headers, /\/index\.html\s+Cache-Control: no-cache/);
assert.match(headers, /\/assets\/\*\s+Cache-Control: public, max-age=31536000, immutable/);
await readFile(new URL('404.html', root));
for (const [file, size] of [['icon-192.png', 192], ['apple-touch-icon.png', 180]]) {
  assert.ok(html.includes(`./${file}`), `Missing icon link: ${file}`);
  const png = await readFile(new URL(file, root));
  assert.deepEqual([...png.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  assert.equal(png.readUInt32BE(16), size);
  assert.equal(png.readUInt32BE(20), size);
}
assert.ok(html.includes('./favicon.ico'));
const ico = await readFile(new URL('favicon.ico', root));
assert.equal(ico.readUInt16LE(2), 1);
assert.equal(ico.readUInt16LE(4), 3);
assert.deepEqual([0, 1, 2].map(i => ico[6 + i * 16]), [16, 32, 48]);
console.log(`Verified browser asset pair: ${path[2]}`);
