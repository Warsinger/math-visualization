import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { repository } from './pages-policy.mjs';
let args;
if (process.env.GITHUB_EVENT_NAME === 'pull_request_target') {
  const event = JSON.parse(await readFile(process.env.GITHUB_EVENT_PATH, 'utf8'));
  if (event.repository.full_name !== repository || event.action !== 'closed' || !event.pull_request.closed_at) throw new Error('Invalid cleanup event');
  args = ['cleanup', '--target', 'preview', '--pr', String(event.number), '--closed-at', event.pull_request.closed_at, '--apply'];
} else {
  const action = process.env.PAGES_ACTION;
  const operations = { 'status-production': ['status', 'production'], 'cleanup-preview': ['cleanup', 'preview'], pause: ['pause', 'production'], resume: ['resume', 'production'], 'undeploy-production': ['undeploy', 'production'] };
  if (!operations[action]) throw new Error('Unknown lifecycle action');
  const [command, target] = operations[action];
  args = [command, '--target', target, '--pr', process.env.PREVIEW_PR || '', '--confirm', process.env.PAGES_CONFIRM || ''];
  if (process.env.PAGES_APPLY === 'true') args.push('--apply');
}
execFileSync(process.execPath, ['scripts/pages.mjs', ...args], { stdio: 'inherit' });
