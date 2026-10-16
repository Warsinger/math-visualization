# Project workflow

- For issue work, use a dedicated branch and create a GitHub pull request targeting `main`; do not push issue implementation directly to `main`.
- Link the relevant issue in the PR, using a closing reference when the PR completes its scope.
- Automatically assign issues being worked on and their pull requests to the user's GitHub account, `Warsinger`.
- Request a review from `Warsinger` on every new pull request, in addition to assigning it to that account.
- Record user-agreed design decisions and scope changes in the relevant GitHub issue as they are made. Keep the current issue description consistent with those decisions, preserve useful discussion history, and link the implementation PR.
- Run appropriate checks and report any remaining validation limits in the PR. For CI changes, inspect the PR's workflow results and resolve failures before handing it back.
- Do not merge a PR without the user's authorization.
