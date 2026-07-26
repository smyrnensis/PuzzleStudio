# Agent Notes

This crate owns browser player/runtime/export behavior, standalone HTML output,
screenshots, themes, audio presentation, and browser-side runtime surfaces.

## Commands

```bash
cargo run -p html-play -- games/spec_2d.puzzle --serve
cargo run -p html-play -- games/spec_2d.puzzle -o /tmp/game.html
cargo run -p html-play -- games/spec_3d.puzzle --screenshot /tmp/spec_3d.png
```

Screenshot commands require Chrome or Chromium on the host. Pass a browser path
or environment variable only when auto-discovery is not enough.

## Runtime Boundaries

Browser adapters play presentation commands such as messages, waits, sounds, and
music. Core remains sound-playback-free, timer-free, and message-state-free.

Standalone exports embed the `puzzle-wasm-player` artifact. Editor previews use
`puzzle-wasm-game`, whose runtime dependency enables the editor debug surface.
Do not substitute the editor artifact into exports or add editor request routes
to the player artifact.

Themes lower to HTML/CSS presentation only; they are not core state.

When behavior is duplicated between Rust/session runtime and standalone
JavaScript, update both or document why one side intentionally differs.

Generated standalone exports belong to output paths such as `games/*.html`; do
not patch generated HTML directly.

## 2D Raster Boundary

Logical visual patterns are resolution-independent presentation data until they
reach the final board canvas. The board renderer must paint those patterns
directly into that canvas and snap shared edges only in final canvas backing
pixels. Do not rasterize a pattern into a per-visual or per-cell bitmap and then
scale that bitmap into the board. That intermediate raster boundary can expose
transparent texels at cell edges and makes adjacent layers quantize
independently.

Keep these cases distinct even though they consume the same logical pattern:

- DOM visual rendering may create a URL-backed bitmap because CSS requires an
  image resource. Name that path as DOM-only and do not reuse it in board canvas
  painting.
- External image visuals are already raster resources and may use `drawImage`.
- Logical pattern visuals in the board canvas must use direct shape painting;
  they must not use `drawImage` with a generated pattern bitmap.
- Level and visual editor grids may use cell DOM because their cells are editing
  controls. This restriction applies to the composed play/preview image, not to
  editor interaction structure.

Optimization must preserve this boundary. Fewer draw calls, shared caches, and
removal of local rounding are not sufficient evidence of correctness. Verify a
multi-cell region with the same full-cell pattern at a fractional fitted cell
size: pixels on both sides of every cell boundary must remain the exact pattern
color. Keep a focused source-contract test that rejects generated-pattern
`drawImage` in the board renderer, and perform a browser screenshot/pixel check
when changing this path.

The responsive screen fit must not apply a compositor `transform` to a finished
board canvas. Scale the screen's presentation/layout coordinate system so DOM
content and the board reach their final CSS geometry before the renderer reads
`getBoundingClientRect()`. That rect intentionally owns final-size, DPR-aware
canvas allocation; it must not describe another raster scaling pass after the
canvas has been painted.
