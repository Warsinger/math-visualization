export const repository = 'Warsinger/math-visualization';
export const projectName = 'math-playground';
export const stateKey = 'MP_PUBLISH_STATE';

export function stateOf(project) {
  const raw = project.deployment_configs?.production?.env_vars?.[stateKey]?.value;
  if (!raw) return { version: 1, paused: false, barrier: 0, previews: {} };
  const state = JSON.parse(raw);
  if (state.version !== 1 || typeof state.paused !== 'boolean' || !Number.isFinite(state.barrier)) throw new Error('Invalid publishing state; refusing to publish');
  state.previews ??= {};
  if (typeof state.previews !== 'object' || Object.entries(state.previews).some(([id, time]) => !/^[1-9][0-9]*$/.test(id) || !Number.isFinite(time))) throw new Error('Invalid preview barriers');
  return state;
}

export function markerOf(deployment) {
  try {
    const text = deployment.deployment_trigger?.metadata?.commit_message;
    return text?.startsWith('MP1:') ? JSON.parse(text.slice(4)) : null;
  } catch { return null; }
}

export function previewMatches(deployment, pr, commits, cutoff = Infinity) {
  if (deployment.environment !== 'preview' || pr.head.repo?.full_name !== repository || pr.base.repo?.full_name !== repository || pr.base.ref !== 'main') return false;
  const metadata = deployment.deployment_trigger?.metadata;
  const marker = markerOf(deployment);
  if (marker) return marker.repo === repository && marker.pr === pr.number && marker.branch === pr.head.ref
    && marker.sha === metadata.commit_hash && metadata.branch === `pr-${pr.number}`
    && Number.isFinite(marker.created) && marker.created >= Date.parse(pr.created_at) && marker.created <= cutoff;
  // Migration of the original #54 previews: exact branch, PR commit membership
  // and PR lifetime, never an alias/name-prefix match.
  return commits.has(metadata?.commit_hash) && metadata?.branch === pr.head.ref && Date.parse(deployment.created_on) >= Date.parse(pr.created_at)
    && Date.parse(deployment.created_on) <= cutoff;
}

export const commentMarker = '<!-- math-playground-preview -->';
export async function upsertComment(api, pr, text, create = true) {
  const comments = await collectPages(async page => ({ result: await api(`/issues/${pr}/comments?per_page=100&page=${page}`) }));
  const existing = comments.find(c => c.user?.login === 'github-actions[bot]' && c.body?.startsWith(commentMarker));
  const body = `${commentMarker}\n${text}`;
  if (existing) {
    if (existing.body !== body) await api(`/issues/comments/${existing.id}`, 'PATCH', { body });
  } else if (create) await api(`/issues/${pr}/comments`, 'POST', { body });
}

export function canPublish(state, started, pr, sha, lastClosed = 0) {
  if (state.paused || started <= state.barrier) return false;
  if (!pr) return true;
  return pr.state === 'open' && pr.head.repo?.full_name === repository && pr.base.repo?.full_name === repository
    && pr.base.ref === 'main' && pr.head.sha === sha && started > Math.max(lastClosed, state.previews?.[pr.number] ?? 0);
}

export async function collectPages(load) {
  const values = [];
  const seen = new Set();
  for (let page = 1; page <= 10000; page++) {
    const { result, totalPages } = await load(page);
    if (!Array.isArray(result)) throw new Error('Invalid pagination response');
    for (const item of result) {
      if (seen.has(item.id ?? item.sha)) throw new Error('Pagination did not advance');
      seen.add(item.id ?? item.sha);
      values.push(item);
    }
    if (!result.length || (totalPages !== undefined ? page >= totalPages : result.length < 100)) return values;
  }
  throw new Error('Pagination limit exceeded');
}

export async function removeDeployments(api, deployments, keep = null) {
  for (const deployment of deployments) {
    if (deployment.id === keep) continue;
    await api(`/deployments/${encodeURIComponent(deployment.id)}?force=true`, 'DELETE', undefined, true);
  }
}

export async function offlineProduction({ pause, upload, getProject, list, remove }, message) {
  await pause();
  await upload(message);
  const project = await getProject();
  if (!stateOf(project).paused) throw new Error('Persistent pause was not retained after offline upload');
  const current = project.canonical_deployment;
  if (current?.deployment_trigger?.metadata?.commit_message !== message || current.environment !== 'production') throw new Error('Offline deployment not canonical; pause retained, no historical deletion attempted');
  await remove((await list()).filter(d => d.environment === 'production'), current.id);
  if ((await list()).some(d => d.environment === 'production' && d.id !== current.id)) throw new Error('Old production URLs remain; pause retained, retry undeploy');
  return current;
}
