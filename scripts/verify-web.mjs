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
console.log(`Verified browser asset pair: ${path[2]}`);
