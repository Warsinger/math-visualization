// Keep each JS/Wasm pair in a content-addressed directory across deployments.
import { createHash } from 'node:crypto';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const input = resolve(root, 'target/web-bindgen');
const output = resolve(root, 'dist/web');
const js = await readFile(resolve(input, 'math_playground.js'));
const wasm = await readFile(resolve(input, 'math_playground_bg.wasm'));
if (wasm.length > 25 * 1024 * 1024) throw new Error('Wasm exceeds the Pages 25 MiB asset limit.');
const hash = createHash('sha256').update(js).update(wasm).digest('hex').slice(0, 24);
const relative = `assets/${hash}`;
const template = await readFile(resolve(root, 'web/index.html'), 'utf8');
if (!template.includes('./math_playground.js')) throw new Error('Bootstrap import was not found.');
// Only clear the verified, generated output directory inside this repository.
if (!output.startsWith(resolve(root, 'dist') + sep)) throw new Error('Invalid generated output path.');
await rm(output, { recursive: true, force: true });
await mkdir(resolve(output, relative), { recursive: true });
await writeFile(resolve(output, relative, 'math_playground.js'), js);
await writeFile(resolve(output, relative, 'math_playground_bg.wasm'), wasm);
await writeFile(resolve(output, 'index.html'), template.replace('./math_playground.js', `./${relative}/math_playground.js`));
await writeFile(resolve(output, '_headers'), await readFile(resolve(root, 'web/_headers')));
// Prevent Pages' implicit SPA fallback from returning HTML for a missing Wasm URL.
await writeFile(resolve(output, '404.html'), '<!doctype html><title>Not found</title><h1>Not found</h1>');
console.log(`Prepared browser assets: ${relative}`);
