// Dependency-free, loopback-only development server. Never executes file content.
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../dist/web/', import.meta.url));
const types = { '.html': 'text/html; charset=utf-8', '.mjs': 'text/javascript', '.css': 'text/css; charset=utf-8', '.png': 'image/png', '.ico': 'image/x-icon', '.js': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm' };
await stat(resolve(root, 'index.html')).catch(() => { throw new Error('Build the web app first; dist/web/index.html is missing.'); });
createServer(async (request, response) => {
  try {
    if (!['GET', 'HEAD'].includes(request.method)) { response.writeHead(405).end(); return; }
    const url = new URL(request.url, 'http://127.0.0.1');
    const path = resolve(root, '.' + decodeURIComponent(url.pathname === '/' ? '/index.html' : url.pathname));
    if (!path.startsWith(resolve(root) + sep)) { response.writeHead(403).end(); return; }
    const data = await readFile(path);
    const extension = path.slice(path.lastIndexOf('.'));
    response.writeHead(200, { 'Content-Type': types[extension] || 'application/octet-stream', 'Cache-Control': 'no-store' });
    response.end(request.method === 'HEAD' ? undefined : data);
  } catch { response.writeHead(404).end('Not found'); }
}).listen(8080, '127.0.0.1', () => console.log('Local playground: http://127.0.0.1:8080 — Ctrl+C stops the server.'));
