# Cloudflare Pages deployment

Target: **https://math-viz.pages.dev/**, Direct Upload project `math-viz`, production branch `main`, in the owner's Cloudflare account. Issue #63 migrates from the original `math-playground` project at https://math-playground-3mq.pages.dev/. That site remains online with its last build and receives no further updates after this migration merges; retirement or redirection is separate scope. The replacement project was created through the dashboard, with all uploads performed through the guarded workflow. The existing account-scoped Pages token is reused, with no additional permissions or paid resources. Scripts verify the exact hostname before operating.

Uses free Pages static hosting and the repository's GitHub Actions allowance. No Functions, storage, custom domain, paid plan or Cloudflare Git build is needed. Account/Actions usage limits still apply. See [Pages limits](https://developers.cloudflare.com/pages/platform/limits/) and [Direct Upload CI](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/).

**FREE is a requirement:** deployment and ongoing operation must remain within included allowances. Do not enable paid plans, metered overage billing or domain purchases. Report a blocking free-tier limit and agree on a free alternative; paid options require separate explicit approval.

## Access setup

1. Create a Cloudflare custom account API token with **Cloudflare Pages: Edit** only in the hosting account. Avoid global keys and unrelated DNS, billing or Workers permissions. This permission manages Pages projects in that account; it is not limited to this GitHub repository. Choose an expiration and rotate before it expires.
2. Add repository Actions secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` directly in GitHub Settings. Never commit credentials or paste them into issues/chat. GitHub cannot reveal a saved secret's value; replace it when rotating.
3. Create the Direct Upload project once through **Workers & Pages → Create application → Continue to Pages → Upload assets**, or `wrangler pages project create math-viz --production-branch main`. Do not connect a second Git-triggered Cloudflare build.
4. Run **Rust checks → Run workflow** on the chosen branch with **deploy** enabled. A successful upload verifies the actual token, account and project access. Dashboard sign-in alone does not validate CI credentials.

The builder GitHub App has no secrets administration permission; the owner installs/rotates secrets directly. The workflow uses a read-only GitHub token and exposes Cloudflare credentials only to its deployment job. PR runs never deploy or receive those credentials.

The owner set the initial token to expire after **one year**. Rotate the GitHub secret before that expiration using the procedure below; an expired token prevents future deployments but does not remove an already published static site.

## Publishing

PRs build/lint Wasm and verify the release JS/Wasm pair, producing a `browser-app` artifact. Existing native checks remain. A push to `main` publishes only after the browser build, native matrix and coverage pass.

Manual **Run workflow** defaults to no deployment. With **deploy** enabled and **pages_action: none**, `main` publishes production; another branch requires **preview_pr** identifying an open same-repository PR targeting main. The run must match its current branch/head. Preview aliases use `pr-<number>` rather than a reusable branch name. There are no automatic PR previews. The existing `ci.yml` has a default-branch dispatch trigger, so its branch picker can select the implementation branch for initial preview validation before merge.

Pinned Wrangler 4.151.1 uploads the previously built artifact without compiling again. Successful publishing writes an **Open the app** link to the workflow summary. For previews, one `<!-- math-playground-preview -->` comment authored by GitHub Actions is created on the PR; later deployments update it in place and identical content is a no-op. Cleanup updates that comment rather than adding another.

Use the workflow for all publishing and lifecycle writes. Direct `wrangler pages deploy` bypasses persistent pause and queue coordination and is no longer a supported manual publishing path. Local scripts support read-only dry runs with process-scoped Cloudflare credentials and a GitHub read token:

```powershell
node scripts/pages.mjs cleanup --target preview --pr 62
node scripts/pages.mjs undeploy --target production
node scripts/pages.mjs status --target production
```

Without `--apply` these print the project, affected IDs/URLs/aliases and current pause state without changing anything. There is no implicit production target. Writes are restricted to serialized workflow jobs. Only generated static assets are uploaded; native history and logs are outside the output folder.

## Undeploy, pause/resume and PR cleanup

In **Rust checks → Run workflow**, leave **deploy** disabled and select a **pages_action**. Lifecycle jobs always check out trusted `main`, even if the dispatch branch differs. These operations become usable after this implementation is merged; before merge, validate their targeting with `node --test scripts/pages.test.mjs`.

| Action | Target and behavior |
| --- | --- |
| `status-production` | Read-only state and production deployment inventory. |
| `cleanup-preview` | Requires `preview_pr`; removes only verified previews for that PR, including older revisions. Production is excluded. |
| `pause` | Persistently blocks publishing; leaves currently served content unchanged. |
| `resume` | Explicitly enables publishing again; does not itself redeploy. Start a new deployment run afterward. |
| `undeploy-production` | First persists pause, publishes maintenance content to main, verifies that it is canonical, then removes every older production deployment. |

All operations default to dry run. Enable **apply** to mutate. Production pause/resume/undeploy also require **confirm: `math-viz:production`**. Actual production undeploy requires separate owner authorization; agreement to the mechanism alone is not authorization to execute it.

**Offline behavior agreed with the owner:** the production hostname and the maintenance deployment URL remain reachable with a static **Application offline** page, HTTP **200**, and `no-store`. Missing app/asset paths return maintenance HTML with **404**. This does not delete the Pages project or token. Old production deployment URLs are deleted so they cannot expose an older app or become rollback targets. Unrelated preview deployments remain available and are listed by the dry run; taking production offline does not remove previews. Already downloaded client code and browser caches cannot be recalled, and CDN deletion may take time to propagate.

The [Pages delete API](https://developers.cloudflare.com/api/resources/pages/subresources/projects/subresources/deployments/methods/delete/) exposes force deletion for non-production aliases; do not attempt to delete the canonical production deployment or assume that deleting one revision removes every app URL. The script keeps the new canonical maintenance deployment and explicitly verifies no older production deployments remain. If any stage fails, it fails visibly, retains the pause, and can be retried. **Undeploy is destructive to production rollback history.** Ordinary rollback remains a separate operation below.

Pause state and run-time barriers are stored in the existing Pages project's `MP_PUBLISH_STATE` plain-text production environment variable, using the same Pages-scoped token. They are public control metadata, not credentials. Automatic and explicit workflow publishing both consult it; normal build/test CI continues. A resumed project rejects uploads from runs started before the resume, so queued old work cannot restore the app. Manual preview cleanup similarly fences runs already queued for that PR; a newly started explicit run may recreate its preview. Do not edit/remove this variable to bypass a pause. The control value has a 4 KiB budget and fails visibly if maintenance is needed rather than enabling paid storage.

Publishing, cleanup and lifecycle operations share one project-wide, non-cancelling concurrency group with `queue: max`. GitHub queues up to 100 waiting jobs; overflow is cancelled visibly and must be retried. PR checks alone retain supersession cancellation. Other parent workflow runs have unique groups, so checks-only dispatches cannot cancel uploads. Cleanup waits for in-flight uploads, then rechecks deployment inventory. A PR closed during upload triggers removal of that upload; a queued publish verifies live PR state, current head and close-event history before upload. Close/reopen cutoffs protect newly reopened previews from older cleanup events. These guarantees cover managed workflows; an administrator's dashboard changes, direct uploads, forced cancellation or external scripts can bypass coordination and need reconciliation.

`pages-close.yml` uses **pull_request_target: closed**, runs trusted main-branch scripts only, and never checks out PR code or consumes PR artifacts. It cleans merged and unmerged PRs. New deployments carry repository/PR/branch/head/run-time metadata. Selection requires exact repository, PR, branch identity and preview environment; original pre-metadata previews additionally require exact branch, PR commit membership and PR lifetime. A reused branch alone never selects a deployment. All API lists paginate; access/errors fail visibly; no matching preview and repeated deletion are successful no-ops. Closing a fork PR cannot target production or another repository's preview.

To restore production after undeploy: explicitly run **resume** with apply and confirmation, then start a **new** normal workflow run on main with deploy enabled (or merge a reviewed change). To restore a manually cleaned preview, start a new deploy run with its open PR number. Do not re-run an old pre-pause/pre-cleanup job: the durable barrier intentionally rejects it.

## Caching and validation

The build hashes JS and Wasm together into `assets/<pair-hash>/`. HTML revalidates (`Cache-Control: no-cache`); hashed assets use a one-year immutable cache. wasm-bindgen resolves Wasm beside its matching JS. Missing assets return 404 instead of the Pages SPA fallback. Extensions preserve JavaScript and `application/wasm` MIME types. No cross-origin-isolation headers are required for this single-threaded build.

After initial deployment and an update:

1. Open the URL in a fresh desktop browser session. Verify startup, helix rendering, typed `x^2`, a sphere, camera drag/zoom, and invalid input preserving the plot. Inspect Console/Network. History is browser-local on the current origin; use the History panel to inspect save warnings.
2. Check HTML and asset cache/MIME headers. Keep an old HTML/asset URL before updating, reload with caching enabled, and confirm a changed pair produces a new URL. Existing loaded tabs keep their app until reload. An old page racing deployment may need reload if its old asset is gone; it cannot silently load a mismatched pair.
3. Record source SHA, deployment ID/URL, browser/version and observed results on the PR. A green upload alone is not interactive validation.

## Rollback and credentials

In Pages **Deployments**, select a known-good **production** deployment and **Rollback to this deployment**. Preview deployments cannot be production rollback targets. Cloudflare restores its uploaded assets without rebuilding. Also revert the bad source through a normal PR, or the next `main` push will publish it again. See [Pages rollbacks](https://developers.cloudflare.com/pages/configuration/rollbacks/). GitHub artifacts last 14 days and are not permanent rollback storage.

For rotation: create an equivalent scoped token, replace the GitHub secret, validate a preview, then revoke the old token. Missing/expired credentials fail deployment without deleting the last good site. Use the lifecycle **pause** action to stop publishing while keeping normal CI running. Fixture tests cover selection, pagination, repeated cleanup, pause/close/reopen races, production exclusion, maintenance ordering/failure, comment upsert and workflow cancellation. Actual production undeploy is not part of PR validation.
