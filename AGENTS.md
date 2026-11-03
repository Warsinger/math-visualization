# Project workflow

- For issue work, use a dedicated branch and create a GitHub pull request targeting `main`; do not push issue implementation directly to `main`.
- Link the relevant issue in the PR, using a closing reference when the PR completes its scope.
- Automatically assign issues being worked on and their pull requests to the user's GitHub account, `Warsinger`.
- Request a review from `Warsinger` on every new pull request, in addition to assigning it to that account.
- Record user-agreed design decisions and scope changes in the relevant GitHub issue as they are made. Keep the current issue description consistent with those decisions, preserve useful discussion history, and link the implementation PR.
- Run appropriate checks and report any remaining validation limits in the PR. For CI changes, inspect the PR's workflow results and resolve failures before handing it back.
- Do not merge a PR without the user's authorization.

# Version workflow

- Include one version bump per issue PR: bug fixes increment patch; features increment minor and reset patch to zero. Review fixes within the same issue PR do not require another bump. Documentation-only follow-ups do not need an independent bump.
- The implementing agent owns the bump in Cargo.toml and Cargo.lock. Recheck against current main before merging; reconcile independent or stacked PRs so each issue advances the version in merge order. A stacked feature can reserve its next minor version ahead of its parent. Major bumps require explicit owner agreement.
- Keep Cargo package versions canonical. Branch/PR/local artifacts get generated dev prerelease labels; clean main CI builds show the normal version. Commit provenance remains embedded even for normal main builds.
- Merging to main automatically tests/builds desktop packages and deploys web. Desktop release tagging and publication require explicit owner authorization and remain manual; do not create a tag or publish a release merely because an issue is complete. Only one current version is supported; no maintenance branches or parallel release lines are required.
