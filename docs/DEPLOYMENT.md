# Cloudflare Pages deployment

Target: **https://math-playground.pages.dev/**, Direct Upload project `math-playground`, production branch `main`, in the owner's Cloudflare account. Confirm the hostname after creation; this configuration does not reserve availability.

Uses free Pages static hosting and the repository's GitHub Actions allowance. No Functions, storage, custom domain, paid plan or Cloudflare Git build is needed. Account/Actions usage limits still apply. See [Pages limits](https://developers.cloudflare.com/pages/platform/limits/) and [Direct Upload CI](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/).

## Access setup

1. Create a Cloudflare custom account API token with **Cloudflare Pages: Edit** only in the hosting account. Avoid global keys and unrelated DNS, billing or Workers permissions. This permission manages Pages projects in that account; it is not limited to this GitHub repository. Choose an expiration and rotate before it expires.
2. Add repository Actions secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` directly in GitHub Settings. Never commit credentials or paste them into issues/chat. GitHub cannot reveal a saved secret's value; replace it when rotating.
3. Create the Direct Upload project once through **Workers & Pages → Create application → Continue to Pages → Upload assets**, or `wrangler pages project create math-playground --production-branch main`. Do not connect a second Git-triggered Cloudflare build.
4. Run **Rust checks → Run workflow** on the chosen branch with **deploy** enabled. A successful upload verifies the actual token, account and project access. Dashboard sign-in alone does not validate CI credentials.

The builder GitHub App has no secrets administration permission; the owner installs/rotates secrets directly. The workflow uses a read-only GitHub token and exposes Cloudflare credentials only to its deployment job. PR runs never deploy or receive those credentials.

## Publishing

PRs build/lint Wasm and verify the release JS/Wasm pair, producing a `browser-app` artifact. Existing native checks remain. A push to `main` publishes only after the browser build, native matrix and coverage pass.

Manual **Run workflow** defaults to no deployment. With **deploy** enabled, `main` publishes production; another branch publishes a Pages preview. There are no automatic PR previews. The existing `ci.yml` already has a default-branch dispatch trigger, so its branch picker can select this implementation branch for initial validation before merge.

Pinned Wrangler 4.151.1 uploads the previously built artifact without compiling again. Deployments are serialized per branch. The upload logs report the deployment URL.

For a trusted local checkout, build using [BROWSER.md](BROWSER.md), then securely supply process environment variables `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` and run:

```powershell
node scripts/verify-web.mjs
npx --yes wrangler@4.151.1 pages deploy dist/web --project-name math-playground --branch main
```

Use another `--branch` for a preview. Wrangler interactive login is also available locally. Only generated static assets are uploaded; native history and logs are outside the output folder.

## Caching and validation

The build hashes JS and Wasm together into `assets/<pair-hash>/`. HTML revalidates (`Cache-Control: no-cache`); hashed assets use a one-year immutable cache. wasm-bindgen resolves Wasm beside its matching JS. Missing assets return 404 instead of the Pages SPA fallback. Extensions preserve JavaScript and `application/wasm` MIME types. No cross-origin-isolation headers are required for this single-threaded build.

After initial deployment and an update:

1. Open the URL in a fresh desktop browser session. Verify startup, helix rendering, typed `x^2`, a sphere, camera drag/zoom, and invalid input preserving the plot. Inspect Console/Network. History remains session-only.
2. Check HTML and asset cache/MIME headers. Keep an old HTML/asset URL before updating, reload with caching enabled, and confirm a changed pair produces a new URL. Existing loaded tabs keep their app until reload. An old page racing deployment may need reload if its old asset is gone; it cannot silently load a mismatched pair.
3. Record source SHA, deployment ID/URL, browser/version and observed results on the PR. A green upload alone is not interactive validation.

## Rollback and credentials

In Pages **Deployments**, select a known-good **production** deployment and **Rollback to this deployment**. Preview deployments cannot be production rollback targets. Cloudflare restores its uploaded assets without rebuilding. Also revert the bad source through a normal PR, or the next `main` push will publish it again. See [Pages rollbacks](https://developers.cloudflare.com/pages/configuration/rollbacks/). GitHub artifacts last 14 days and are not permanent rollback storage.

For rotation: create an equivalent scoped token, replace the GitHub secret, validate a preview, then revoke the old token. Missing/expired credentials fail deployment without deleting the last good site. To pause publishing, change the deploy condition through a PR; temporarily disabling the entire Rust checks workflow also pauses its other CI jobs.
