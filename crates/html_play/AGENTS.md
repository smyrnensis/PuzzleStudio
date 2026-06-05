# Agent Notes

This crate owns browser player/runtime/export behavior, standalone HTML output,
screenshots, themes, audio presentation, and browser-side runtime surfaces.

## Commands

```bash
cargo run -p html-play -- games/spec_2d.puzzle --serve
cargo run -p html-play -- games/spec_2d.puzzle -o /tmp/game.html
cargo run -p html-play -- games/spec_3d.puzzle3 --screenshot /tmp/spec_3d.png
```

Screenshot commands require Chrome or Chromium on the host. Pass a browser path
or environment variable only when auto-discovery is not enough.

## Runtime Boundaries

Browser adapters play presentation commands such as messages, waits, sounds, and
music. Core remains sound-playback-free, timer-free, and message-state-free.

Themes lower to HTML/CSS presentation only; they are not core state.

When behavior is duplicated between Rust/session runtime and standalone
JavaScript, update both or document why one side intentionally differs.

Generated standalone exports belong to output paths such as `games/*.html`; do
not patch generated HTML directly.
