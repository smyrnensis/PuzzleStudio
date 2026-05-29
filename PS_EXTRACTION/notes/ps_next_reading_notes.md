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
- 2026-05-29: local checkout is present at `upstream/PuzzleScriptNext`.
  `src/js/levels3d.js`, parser `levels3` handling, compiler `levels3ToArray`, and
  `test/levels3d.test.js` are present. The tests pass with
  `node upstream/PuzzleScriptNext/test/levels3d.test.js`.

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

### 2026-05-29

- Read `src/js/levels3d.js`: contains `parseLevels3`, `parseLevel3`,
  `coordToIndex3`, and `indexToCoord3`.
- Read `src/js/compiler.js` around `levels3ToArray` and
  `level3FromParsedSource`: `LEVELS3` can lower to a 3D level-shaped object.
- Read `src/js/engine.js` around direction masks and `Level`: the runtime is
  still 2D-shaped (`width`, `height`, `n_tiles = width * height`) and movement
  deltas are 2D.
- Read `test/levels3d.test.js`: current tests cover parser storage, lowering,
  glyph validation, background fill, and coordinate round trips.
- Conclusion: the next step is a runtime boundary gate, not a 3D renderer.
- Added the first boundary gate in `src/js/compiler.js`: `LEVELS3` now produces
  an explicit unsupported-runtime message instead of falling through as missing
  2D levels.
- Added the first engine shape gate in `src/js/engine.js`: `Level` can carry
  optional `depth`/`is3d`, `deltaPositionIndex3` computes depth-aware offsets,
  and named 3D direction deltas are isolated from the existing 2D movement mask
  table.
- Architecture direction updated: use a shared upper layer that chooses between
  separate 2D and 3D lower-level core runtimes/renderers. The existing engine
  remains the 2D core runtime; 3D should proceed as a small separate runtime
  path rather than an in-place rewrite of the 2D engine.
- Traced 2D metadata usage across `parser.js`, `compiler.js`, `engine.js`,
  `graphics.js`, and `inputoutput.js`, then recorded 3D contract slots in
  `METADATA_CONTRACT_3D.md`.
