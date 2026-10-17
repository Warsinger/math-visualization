// All writes run inside the shared GitHub lifecycle concurrency queue.
import { mkdtemp, writeFile, appendFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFile } from 'node:child_process';
import { repository, projectName, stateKey, stateOf, canPublish, previewMatches, collectPages, removeDeployments, upsertComment, offlineProduction, parseOptions } from './pages-policy.mjs';

const args = process.argv.slice(2);
const command = args.shift();
const options = parseOptions(args);
const value = name => options[name.replace(/^--/, '')];
const apply = options.apply;
const target = value('--target');
if (!['status', 'publish', 'pause', 'resume', 'undeploy', 'cleanup'].includes(command)) throw new Error('Specify status, publish, pause, resume, undeploy or cleanup');
if (!['production', 'preview'].includes(target)) throw new Error('Explicit --target production|preview required');
if (apply && (process.env.GITHUB_ACTIONS !== 'true' || process.env.PAGES_SERIALIZED !== 'true')) throw new Error('Writes require the serialized GitHub workflow; local commands are dry-run only');
if (apply && target === 'production' && command !== 'publish' && value('--confirm') !== 'math-viz:production') throw new Error('Production confirmation must be math-viz:production');
if (target === 'preview' && ['pause', 'resume', 'undeploy'].includes(command)) throw new Error('Use cleanup for previews');
const account = process.env.CLOUDFLARE_ACCOUNT_ID;
if (!/^[a-f0-9]{32}$/.test(account ?? '') || !process.env.CLOUDFLARE_API_TOKEN) throw new Error('Missing/invalid Cloudflare credentials');
const prefix = `https://api.cloudflare.com/client/v4/accounts/${account}/pages/projects/${projectName}`;
async function request(url, token, method = 'GET', body, missingOkay = false) {
  const response = await fetch(url, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json', Accept: 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
  if (response.status === 404 && missingOkay) return null;
  const data = await response.json();
  if (!response.ok || data.success === false) throw new Error(`API ${method} failed: HTTP ${response.status} at ${new URL(url).pathname}`);
  return data;
}
const cf = (path = '', method, body, missingOkay) => request(prefix + path, process.env.CLOUDFLARE_API_TOKEN, method, body, missingOkay);
const gh = (path, method, body) => request(`https://api.github.com/repos/${repository}${path}`, process.env.GITHUB_TOKEN, method, body);
const ghList = path => collectPages(async page => ({ result: await gh(`${path}${path.includes('?') ? '&' : '?'}per_page=100&page=${page}`) }));
const list = () => collectPages(async page => { const data = await cf(`/deployments?page=${page}`); return { result: data.result, totalPages: data.result_info?.total_pages, pageSize: data.result_info?.per_page ?? null }; });
let project = (await cf()).result;
if (project.subdomain !== `${projectName}.pages.dev`) throw new Error('Unexpected Pages hostname; refusing to operate');
if (project.production_branch !== 'main') throw new Error('Production branch must be main');
let state = stateOf(project);
const deployments = await list();
const summary = deployments => deployments.map(d => ({ id: d.id, environment: d.environment, url: d.url, aliases: d.aliases }));
let pr;
let commits;
let cutoff = Infinity;
if (target === 'preview') {
  const number = value('--pr');
  if (!/^[1-9][0-9]*$/.test(number ?? '')) throw new Error('Preview requires explicit --pr number');
  pr = await gh(`/pulls/${number}`);
  if (pr.head.repo?.full_name !== repository || pr.base.repo?.full_name !== repository || pr.base.ref !== 'main') {
    if (command === 'cleanup') { console.log('No managed same-repository preview: no-op'); process.exit(0); }
    throw new Error('Preview PR must be from this repository and target main');
  }
  commits = new Set((await ghList(`/pulls/${number}/commits`)).map(c => c.sha));
  if (value('--closed-at')) {
    cutoff = Date.parse(value('--closed-at'));
    if (!Number.isFinite(cutoff)) throw new Error('Invalid close-event cutoff');
  }
}
const selected = target === 'production' ? deployments.filter(d => d.environment === 'production') : deployments.filter(d => previewMatches(d, pr, commits, cutoff));
console.log(JSON.stringify({ project: projectName, command, target, dryRun: !apply, paused: state.paused, affected: summary(selected), unaffectedPreviewUrls: target === 'production' ? summary(deployments.filter(d => d.environment === 'preview')) : undefined }, null, 2));
async function report(text) {
  console.log(text);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, `${text}\n`);
}
if (!apply || command === 'status') process.exit(0);
async function saveState(paused, productionBarrier = true) {
  // Patch only this public control variable; never echo other project variables.
  state = { ...state, version: 1, paused, barrier: productionBarrier ? Date.now() : state.barrier };
  if (JSON.stringify(state).length > 4096) throw new Error('Publishing control state exceeds its size budget; manual maintenance required, no paid upgrade');
  await cf('', 'PATCH', { deployment_configs: { production: { env_vars: { [stateKey]: { type: 'plain_text', value: JSON.stringify(state) } } } } });
  project = (await cf()).result;
  if (JSON.stringify(stateOf(project)) !== JSON.stringify(state)) throw new Error('Publishing state was not persisted');
}
async function upload(directory, branch, message) {
  await new Promise((resolve, reject) => {
    const child = execFile('npx', ['--yes', 'wrangler@4.151.1', 'pages', 'deploy', directory, '--project-name', projectName, '--branch', branch, '--commit-hash', process.env.GITHUB_SHA, '--commit-message', message], { env: process.env }, error => error ? reject(new Error('Wrangler upload failed; publishing state retained')) : resolve());
    child.stdout.pipe(process.stdout);
    child.stderr.pipe(process.stderr);
  });
}
if (command === 'pause' || command === 'resume') {
  if (target !== 'production') throw new Error('Pause/resume requires production target');
  await saveState(command === 'pause');
  console.log('Publishing state saved; resume requires a newly started deploy run to publish.');
} else if (command === 'cleanup') {
  if (target !== 'preview') throw new Error('Cleanup must target preview; production uses undeploy');
  // A manual cleanup must also fence uploads already queued for an open PR.
  if (!Number.isFinite(cutoff)) {
    state.previews[pr.number] = Math.max(state.previews[pr.number] ?? 0, Date.now());
    await saveState(state.paused, false);
  } else if (state.previews[pr.number] !== undefined && state.previews[pr.number] <= cutoff) {
    // GitHub close history now fences this PR. Retire obsolete manual barriers
    // instead of accumulating a control entry for every closed/no-preview PR.
    delete state.previews[pr.number];
    await saveState(state.paused, false);
  }
  await removeDeployments(cf, selected);
  const inventory = await list();
  const remaining = inventory.filter(d => previewMatches(d, pr, commits, cutoff));
  if (remaining.length) throw new Error('Matching previews remain after cleanup; retry visibly');
  console.log('Preview cleanup complete (including already-gone no-op).');
  await report(`Preview cleanup for PR #${pr.number}: ${selected.length} matching deployment(s) removed; production preserved.`);
  const retained = inventory.filter(d => previewMatches(d, pr, commits)).sort((a, b) => Date.parse(b.created_on) - Date.parse(a.created_on))[0];
  const comment = retained
    ? `[Open the playground preview](${retained.url})\n\nA newer reopened-PR preview was preserved by cleanup.`
    : 'Preview removed. An open PR can explicitly deploy a fresh preview.';
  await upsertComment(gh, pr.number, comment, false);
} else if (command === 'undeploy') {
  const directory = await mkdtemp(join(tmpdir(), 'math-playground-offline-'));
  const html = '<!doctype html><meta charset="utf-8"><title>Application offline</title><h1>Application offline</h1><p>The playground is temporarily unavailable.</p>';
  await writeFile(`${directory}/index.html`, html);
  await writeFile(`${directory}/404.html`, html);
  await writeFile(`${directory}/_headers`, '/*\n  Cache-Control: no-store\n  X-Robots-Tag: noindex\n');
  const message = `MP-OFFLINE:${process.env.GITHUB_RUN_ID}:${process.env.GITHUB_RUN_ATTEMPT}`;
  const current = await offlineProduction({ pause: () => saveState(true), upload: () => upload(directory, 'main', message), getProject: async () => (await cf()).result, list, remove: (items, keep) => removeDeployments(cf, items, keep) }, message);
  console.log(JSON.stringify({ offlineUrl: current.url, productionHostname: project.subdomain, status: 'maintenance HTML 200; missing assets 404', previewUrlsUnaffected: true }));
} else if (command === 'publish') {
  const run = await gh(`/actions/runs/${process.env.GITHUB_RUN_ID}`);
  const started = Date.parse(run.created_at);
  if (!Number.isFinite(started)) throw new Error('Cannot verify run creation time');
  let lastClosed = 0;
  if (pr) {
    lastClosed = Math.max(0, ...(await ghList(`/issues/${pr.number}/events`)).filter(e => e.event === 'closed').map(e => Date.parse(e.created_at)));
    if (run.head_branch !== pr.head.ref || process.env.GITHUB_REF_NAME !== pr.head.ref) throw new Error('Run branch does not match PR');
    pr = await gh(`/pulls/${pr.number}`);
  } else {
    if (process.env.GITHUB_REF !== 'refs/heads/main') throw new Error('Production publish requires main');
    if ((await gh('/git/ref/heads/main')).object.sha !== process.env.GITHUB_SHA) { console.log('Stale main run: publishing skipped'); process.exit(0); }
  }
  state = stateOf((await cf()).result);
  if (!canPublish(state, started, pr, process.env.GITHUB_SHA, lastClosed)) { console.log('Paused, stale, closed or superseded run: publishing skipped'); process.exit(0); }
  const marker = `MP1:${JSON.stringify({ repo: repository, pr: pr?.number ?? null, branch: process.env.GITHUB_REF_NAME, sha: process.env.GITHUB_SHA, created: started, run: process.env.GITHUB_RUN_ID, attempt: process.env.GITHUB_RUN_ATTEMPT })}`;
  await upload('dist/web', pr ? `pr-${pr.number}` : 'main', marker);
  const uploaded = (await list()).find(d => d.deployment_trigger?.metadata?.commit_message === marker && d.environment === (pr ? 'preview' : 'production'));
  if (!uploaded || !/^https:\/\/[a-z0-9.-]+\.pages\.dev\/?$/.test(uploaded.url)) throw new Error('Cannot verify uploaded app URL');
  // A close during the upload is safe even before the queued close job runs.
  if (pr && (await gh(`/pulls/${pr.number}`)).state !== 'open') {
    await removeDeployments(cf, (await list()).filter(d => d.environment === 'preview' && d.deployment_trigger?.metadata?.commit_message === marker));
    console.log('PR closed during upload: uploaded preview removed.');
    await upsertComment(gh, pr.number, 'Preview removed because the PR closed during deployment.', false);
  } else {
    await report(`### Playground deployment\n\n[Open the app](${uploaded.url})\n\nSource: ${process.env.GITHUB_SHA}`);
    if (pr) await upsertComment(gh, pr.number, `[Open the playground preview](${uploaded.url})\n\nSource: \`${process.env.GITHUB_SHA}\`. Updated in place on deployment; removed on PR close.`);
  }
}
