# PS Next Reading Notes

Purpose: record the exact PS Next files read, the behavior observed, and the
3D integration points inferred from local source inspection.

Do not treat impact estimates as confirmed until the corresponding PS Next
file has been read in the local checkout.

## Checkout

- Expected local path: `PS_EXTRACTION/upstream/PuzzleScriptNext`
- Upstream: `https://github.com/david-pfx/PuzzleScriptNext`
- License observed from GitHub: MIT
- 2026-05-27: full `git clone`, shallow `git clone --depth 1`, and GitHub
  source zip download were attempted from this environment. All failed because
  the network connection reset before the full payload completed. The local
  checkout is therefore not present yet.

## First Files To Inspect

- `src/js/parser.js`
- `src/js/compiler.js`
- `src/js/engine.js`
- `src/js/graphics.js`
- `src/js/inputoutput.js`
- `src/js/editor.js`
- `src/js/buildStandalone.js`
- `src/js/toolbar.js`
- `src/Documentation/`

## Reading Log

Add dated notes here as files are inspected.
