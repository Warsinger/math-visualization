# Versions and releases

## The normal issue workflow

The implementing agent includes one version bump in every issue PR, in Cargo.toml and Cargo.lock together. A bug fix increments patch (0.2.0 to 0.2.1); a feature increments minor and resets patch (0.2.1 to 0.3.0). Review fixes within the same issue keep its planned version. Documentation-only follow-ups do not need another bump. Major increments require owner agreement.

Before merging, the agent rechecks the version against current main and resolves conflicts. Stacked PRs reserve versions in merge order: versioning #85 is 0.2.0, and About #86 based on it is 0.3.0. The identity CI gate rejects unchanged/backward versions. It cannot infer bug versus feature scope, so the agent owns the exact patch/minor increment and checks it again at merge time. Forward versions ahead of main are allowed for stacked work, including a bug fix following a feature; this does not authorize skipping the prescribed per-issue bump. There is no automatic version commit after merge and no bump for every CI run. AGENTS.md records the agent's responsibilities for future chats.

Cargo.toml package.version is the authoritative canonical SemVer. Windows numeric components must fit 16-bit fields; Cargo versions must not include +build metadata. Changes to Cargo.toml require updating the root package version in Cargo.lock.

## What a build displays

| Build | Example display version | Meaning |
| --- | --- | --- |
| PR or feature-branch CI | 0.2.0-dev.ci.123.1+g0123456789ab | Development prerelease; downloadable for testing, not an official desktop release. |
| Local build | 0.2.0-dev.local+g0123456789ab.dirty | Development prerelease; .dirty appears for uncommitted changes. No GitHub access required. |
| Clean main CI, push or manual run | 0.2.0 | Current main/web version; channel is main, not a published desktop release. |
| Matching clean release tag v0.2.0 | 0.2.0 | Release candidate for owner-controlled desktop publication; channel is release. |

Source archives without Git use gunknown. Local builds stay development prereleases even on main. Optional explicit prerelease Cargo versions such as 0.3.0-rc.1 remain prereleases and receive .dev suffixes on development builds. A real SemVer hyphen prerelease is used; +commit metadata alone is not a prerelease. Dirty main CI builds cannot get an ordinary stable label. Every build still embeds full commit, dirty state, channel, target, profile and compiler; timestamps are omitted for reproducibility. PR provenance is the tested merge commit, which may differ from the branch head.

## What merging does

A merge to main tests and builds all desktop packages and automatically deploys the web app after checks pass. It does not create a tag or publish a GitHub desktop release. Web can therefore be newer than the latest desktop download. Both use the same canonical version source. We support one current version, without maintenance branches or parallel release lines.

## Publishing a desktop release

1. Obtain explicit owner authorization for the desktop release. Ordinary issue completion or merge approval is not release-publication authorization.
2. Choose a reviewed main commit and create/push an annotated tag exactly v followed by its Cargo version. Do not reuse or move existing tags.
3. Tag CI validates exact version match, main ancestry and absence of an existing GitHub release for that tag, then runs the full build/test matrix. API failures fail closed. Tagging does not deploy web or publish desktop.
4. After checks pass, download that tag run's versioned native archives/checksums and browser artifact if needed. Inspect build-info.json; create the GitHub release manually for the same tag, uploading unchanged artifacts. Mark explicit rc/beta versions as prereleases. Never upload feature-branch/dev artifacts as official releases.

The workflow has no release-publication write permission (existing Pages permissions are unchanged). No tag/release is created by implementing this change. Git rejects ordinary duplicate tag pushes; an administrator must configure protected/immutable tags to prohibit forced replacement. We do not alter repository administration settings. An existing published release blocks rebuilding its tag through this workflow; retain the original artifacts rather than starting another supported release line.

## Metadata consistency

The app uses embedded identity. Native --build-info prints exact compiled JSON without opening a window. Packaging queries that executable and checks Cargo version, revision, target and release profile. Windows string metadata matches display_version; numeric version is major.minor.patch.0. Native archive/artifact names include display_version and target. JSON comes from the binary, not the packaging checkout.

Browser build-info.json comes from the Cargo output that produced the Wasm. It is hashed with the complete runtime; verification checks its bytes are embedded in that Wasm, and build_info is exported for diagnostics. The version label is compiled into the app so cached code identifies itself. HTML revalidation plus immutable paths updates code and metadata together. The internal browser-app CI artifact name remains unchanged for deployment.

Validation covers local/branch/main/tag label modes, dirty/unknown fallbacks, invalid tags, forward version checks, duplicate release/API failures, browser pairing and native packages on every CI target. No live release/tag is created for testing.
