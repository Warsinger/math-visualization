# Compact shared footer

The owner prefers the newer footer used by Game of Life and Rover. This page follows that compact visual pattern: muted 12px text, left-aligned links, dot separators and no divider/promotional paragraph. Match the footer to the existing page container (900px here) rather than changing the app layout.

## Content

- All apps: https://eelwehttam.pages.dev/
- Source: https://github.com/Warsinger/math-visualization
- Report an issue: https://github.com/Warsinger/math-visualization/issues

Each link retains the existing safe new-tab behavior, with noopener/noreferrer and an accessible new-tab announcement, so navigation preserves the active app. Links remain underlined and have a visible keyboard focus outline. Keep existing attribution elsewhere; do not invent asset credits or licensing claims.

The footer stays in normal document flow outside the canvas/iframe. It may wrap on narrow screens. In Math Viz's full-height flex page, retain flex: none on the footer and flex: 1; min-height: 0 on the canvas so the footer does not cover plotting controls. Each repository owns this small static copy; no shared JavaScript, remote stylesheet, build dependency or coordinated deployment is needed.

## Markup

```html
<footer class="project-footer">
  <a href="https://eelwehttam.pages.dev/" target="_blank" rel="noopener noreferrer" aria-label="All apps (opens in a new tab)">All apps</a> ·
  <a href="https://github.com/Warsinger/math-visualization" target="_blank" rel="noopener noreferrer" aria-label="Source (opens in a new tab)">Source</a> ·
  <a href="https://github.com/Warsinger/math-visualization/issues" target="_blank" rel="noopener noreferrer" aria-label="Report an issue (opens in a new tab)">Report an issue</a>
</footer>
```

## Styles

```css
.project-footer { box-sizing: border-box; flex: none; width: 100%; max-width: 900px; margin: 0 auto; padding: 0 16px 24px; color: #8b9db4; font: 12px/1.7 system-ui, sans-serif; text-align: left; }
.project-footer a { color: inherit; text-underline-offset: 4px; }
.project-footer a:hover { text-decoration-thickness: 2px; }
.project-footer a:focus-visible { outline: 2px solid currentColor; outline-offset: 4px; border-radius: 2px; }
```

Verify all three destinations, keyboard focus, desktop/narrow wrapping and packaged output. The native app and inner runtime document do not need a website navigation footer.
