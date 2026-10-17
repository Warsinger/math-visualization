import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { repository, stateOf, canPublish, previewMatches, collectPages, removeDeployments, upsertComment, commentMarker, offlineProduction, parseOptions } from './pages-policy.mjs';
const pr = { number: 62, state: 'open', created_at: '2026-10-01T00:00:00Z', head: { ref: 'codex/feature', sha: 'new', repo: { full_name: repository } }, base: { ref: 'main', repo: { full_name: repository } } };
const start = Date.parse('2026-10-16T00:00:00Z');
const deployment = (overrides = {}) => ({ id: 'one', environment: 'preview', created_on: '2026-10-16T00:01:00Z', deployment_trigger: { metadata: { branch: 'pr-62', commit_hash: 'old', commit_message: `MP1:${JSON.stringify({ repo: repository, pr: 62, branch: 'codex/feature', sha: 'old', created: start })}` } }, ...overrides });
test('dry-run options cannot be turned into apply by a string input', () => {
  assert.equal(parseOptions(['--target', 'production', '--pr', '']).apply, false);
  assert.throws(() => parseOptions(['--target', 'production', '--pr', '--apply']));
  assert.equal(parseOptions(['--target', 'preview', '--pr', '62', '--apply']).apply, true);
});
test('preview identity excludes production, unrelated PR/repository/branch and handles old force-pushed revisions', () => {
  assert.ok(previewMatches(deployment(), pr, new Set()));
  assert.ok(!previewMatches(deployment({ environment: 'production' }), pr, new Set(['old'])));
  assert.ok(!previewMatches(deployment(), { ...pr, number: 63 }, new Set(['old'])));
  assert.ok(!previewMatches(deployment(), { ...pr, head: { ...pr.head, ref: 'reused' } }, new Set(['old'])));
  assert.ok(!previewMatches(deployment(), { ...pr, head: { ...pr.head, repo: { full_name: 'other/repo' } } }, new Set(['old'])));
});
test('close cutoff includes in-flight old upload but excludes post-reopen run', () => {
  assert.ok(previewMatches(deployment(), pr, new Set(), start + 1000));
  assert.ok(!previewMatches(deployment(), pr, new Set(), start - 1));
});
test('legacy previews require exact branch, PR commit membership and lifetime', () => {
  const legacy = deployment({ deployment_trigger: { metadata: { branch: pr.head.ref, commit_hash: 'old' } } });
  assert.ok(previewMatches(legacy, pr, new Set(['old'])));
  assert.ok(!previewMatches(legacy, pr, new Set()));
  assert.ok(!previewMatches({ ...legacy, created_on: '2020-01-01' }, pr, new Set(['old'])));
});
test('pause/resume barrier rejects queued uploads, stale heads and closed/reopened old runs', () => {
  const active = { paused: false, barrier: start - 1 };
  assert.ok(canPublish(active, start, pr, 'new'));
  for (const [state, p, sha, closed] of [[{ ...active, paused: true }, pr, 'new', 0], [{ ...active, barrier: start }, pr, 'new', 0], [active, { ...pr, state: 'closed' }, 'new', 0], [active, pr, 'old', 0], [active, pr, 'new', start]]) assert.ok(!canPublish(state, start, p, sha, closed));
  assert.ok(canPublish(active, start + 2, pr, 'new', start + 1));
});
test('state defaults safely and corrupt persisted state fails closed', () => {
  assert.deepEqual(stateOf({}), { version: 1, paused: false, barrier: 0, previews: {} });
  assert.throws(() => stateOf({ deployment_configs: { production: { env_vars: { MP_PUBLISH_STATE: { value: '{bad' } } } } }));
});
test('manual preview cleanup fences queued open-PR uploads but permits a new explicit run', () => {
  const state = { paused: false, barrier: 0, previews: { 62: start } };
  assert.ok(!canPublish(state, start, pr, 'new'));
  assert.ok(canPublish(state, start + 1, pr, 'new'));
});
test('pagination covers all pages and fails on repeated/error pages', async () => {
  const all = await collectPages(async page => ({ result: [{ id: page }], totalPages: 3 }));
  assert.equal(all.length, 3);
  await assert.rejects(collectPages(async () => ({ result: [{ id: 1 }], totalPages: 3 })), /did not advance/);
  await assert.rejects(collectPages(async () => { throw new Error('HTTP 403'); }), /403/);
});
test('Cloudflare default-sized pages do not truncate when total-pages metadata is absent', async () => {
  const visited = [];
  const all = await collectPages(async page => {
    visited.push(page);
    return { result: page <= 2 ? Array.from({ length: 25 }, (_, i) => ({ id: (page - 1) * 25 + i })) : [], pageSize: null };
  });
  assert.equal(all.length, 50);
  assert.deepEqual(visited, [1, 2, 3]);
});
test('cleanup is repeatable/no-op, preserves explicitly retained deployment and uses force for aliases', async () => {
  const calls = [];
  const api = async (...args) => { calls.push(args); return null; };
  await removeDeployments(api, []);
  assert.equal(calls.length, 0);
  await removeDeployments(api, [{ id: 'keep' }, { id: 'gone' }], 'keep');
  assert.deepEqual(calls[0], ['/deployments/gone?force=true', 'DELETE', undefined, true]);
  await removeDeployments(api, [{ id: 'gone' }]);
  assert.equal(calls.length, 2);
});
test('one PR comment is created, updated in place, and unchanged content is a no-op', async () => {
  const comments = [];
  let writes = 0;
  const api = async (path, method, data) => {
    if (!method) return comments;
    writes++;
    if (method === 'POST') comments.push({ id: 1, user: { login: 'github-actions[bot]' }, body: data.body });
    else comments[0].body = data.body;
  };
  await upsertComment(api, 62, 'link A');
  await upsertComment(api, 62, 'link A');
  assert.equal(writes, 1);
  await upsertComment(api, 62, 'link B');
  assert.equal(comments.length, 1);
  assert.equal(comments[0].body, `${commentMarker}\nlink B`);
  await upsertComment(api, 62, 'removed', false);
  assert.equal(comments.length, 1);
});
test('workflow parent cancellation only affects PR checks; all mutations share non-cancelling queue', async () => {
  const ci = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const close = await readFile(new URL('../.github/workflows/pages-close.yml', import.meta.url), 'utf8');
  assert.match(ci, /cancel-in-progress: \$\{\{ github.event_name == 'pull_request' \}\}/);
  assert.match(ci, /github.event_name == 'pull_request' && github.ref \|\| github.run_id/);
  for (const text of [ci, close]) assert.match(text, /group: cloudflare-math-playground-lifecycle\s+cancel-in-progress: false\s+queue: max/);
  assert.match(close, /pull_request_target:\s+types: \[closed\]/);
  assert.match(close, /ref: main/);
  assert.ok(!close.includes('head.sha') && !close.includes('download-artifact'));
});
test('production offline pauses first, verifies maintenance, removes all old production but no previews', async () => {
  const order = [];
  let items = [{ id: 'old', environment: 'production' }, { id: 'offline', environment: 'production' }, { id: 'preview', environment: 'preview' }];
  const maintenance = { id: 'offline', environment: 'production', deployment_trigger: { metadata: { commit_message: 'offline-marker' } } };
  const ops = {
    pause: async () => order.push('pause'), upload: async () => order.push('upload'),
    getProject: async () => ({ canonical_deployment: maintenance, deployment_configs: { production: { env_vars: { MP_PUBLISH_STATE: { value: JSON.stringify({ version: 1, paused: true, barrier: 1 }) } } } } }),
    list: async () => items,
    remove: async (selected, keep) => { order.push('remove'); assert.deepEqual(selected.map(d => d.id), ['old', 'offline']); items = items.filter(d => !selected.includes(d) || d.id === keep); }
  };
  await offlineProduction(ops, 'offline-marker');
  assert.deepEqual(order, ['pause', 'upload', 'remove']);
  assert.deepEqual(items.map(d => d.id), ['offline', 'preview']);
  order.length = 0;
  await assert.rejects(offlineProduction({ ...ops, upload: async () => { throw new Error('upload failed'); } }, 'offline-marker'), /upload failed/);
  assert.deepEqual(order, ['pause']);
  await assert.rejects(offlineProduction(ops, 'wrong-marker'), /not canonical/);
});
