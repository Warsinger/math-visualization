# Shared project footer

Math Viz #74 follows the portable footer defined in [Tower Defense PR #24](https://github.com/Warsinger/tower-defense-go/pull/24), checked against revision c28cebb2097fb335cba5fadd7aca8ff6ce56d934 and its [design guide](https://github.com/Warsinger/tower-defense-go/blob/c28cebb2097fb335cba5fadd7aca8ff6ce56d934/docs/WEB_DESIGN.md).

Both apps use the same footer text, destination, accessible name, decorative arrow and four CSS rules. The underlined link opens https://eelwehttam.pages.dev/ in a new tab with noopener/noreferrer, preserving the active app. Its keyboard focus outline remains visible. Each repository owns a small static copy; no runtime dependency joins the sites.

The footer is in normal document flow outside the canvas. Math Viz uses a full-height flex page: the status bar and footer have flex: none, while the canvas takes the remaining height with min-height: 0. Text wraps on narrow screens. The app workspace scrolls within the canvas as needed; the footer is never an overlay.

For future changes, compare the footer HTML and CSS in both repositories. Inspect desktop and narrow layouts, keyboard focus, new-tab behavior and the packaged output. Native apps need no website navigation footer.
