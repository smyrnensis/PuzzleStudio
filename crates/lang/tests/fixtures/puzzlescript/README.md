# PuzzleScript Import Fixtures

This folder pins PuzzleScript-to-canonical `.puzzle` mappings.

Current vanilla PuzzleScript import scope:

- `OBJECTS` paragraphs define object names and PS-style color/pattern sprites.
- `OBJECTS` one-character shorthand such as `BlueCrate B` becomes a canonical `levels { legend { ... } }` row.
- `LEGEND` rows become canonical `levels { legend { ... } }` rows. `and` becomes a same-cell object list.
- `LEGEND` property aliases such as `crate = crate1 or crate2 or crate3` become canonical `group` rows.
- A PuzzleScript object named `Background` is treated as PS's special background object: it remains a normal canonical object/sprite, and the importer adds `on_level_start { once_all [ no Background ] -> [ Background ] }` so every level cell gets background on load.
- `COLLISIONLAYERS` rows become canonical `layers`; generated output does not use `objects {}`.
- `COLLISIONLAYERS` property aliases are expanded to their concrete objects.
- `WINCONDITIONS` rows are copied when they match canonical condition forms such as `all Target on Crate`.
- Empty PS `WINCONDITIONS` sections are omitted from canonical output.
- `RULES` rows are copied as canonical rules. Prefixless rules containing PS movement markers (`>`, `<`, `^`, `v`) rely on canonical implicit cardinal expansion.
- PS's special `Player` movement is represented by inserting `input directions [ Player ] -> [ Player{>} ]`.
- PS movement markers use the canonical anonymous movement scratch, and the existing built-in `move` routine resolves the movement phase.
- If `again` appears, importer emits the canonical `again` rule effect. Runtime treats it as a request for a no-input follow-up turn after the current turn commits. It does not resend the previous key or semantic input; it reruns the same puzzle rule entrypoint with no input. Standalone HTML spaces automatic turns by `defaultAgainMs` and exposes each turn's `sfx` emissions separately.
- PS `late` rules are emitted after `move` inside that loop.
- PS `moving` / `stationary` qualifiers become anonymous movement scratch predicates such as `Crate{directions}` and `Crate{no directions}` on LHS; RHS `stationary` is emitted as the bare object.
- Simple PS `SOUNDS` rows such as `sfx0 12345` become canonical `sounds { sfx sfx0 seed=12345 type=puzzlescript }`; PS rule suffixes such as `SFX0` become `sfx sfx0`.
- `LEVELS` splits blank-line-separated PS levels into canonical unnamed levels.
- A default `scene title` and `scene playing` are generated. The title scene defines `confirm <- Enter Space x`; the `Play` button emits `input confirm`, so keyboard confirm and button click use the same scene rule before `start levels in playing`.
- Generated sprite entries and level bodies use brace-less canonical forms. Generated output uses spaces, not tabs.

Pinned samples:

- `basic_sokoban.ps` is a local minimal Sokoban import fixture.
- `official_sumo.ps` is copied from the official PuzzleScript repo's `src/demo/sumo.txt`; it covers a disconnected pattern rule.
- `official_twolittlecrates1.ps` is copied from the official PuzzleScript repo's `src/demo/twolittlecrates1.txt` as a known next-scope sample. It uses object character shorthand and property aliases.
- `official_simple_block_sliding.ps` is copied from increpare's "Simple Block Sliding Game" PuzzleScript sample gist: https://gist.github.com/increpare/0c310ba2559b8d27973601ffe43c0478. It covers object character shorthand, property aliases, `again` lowering, `late`, `moving` / `stationary` qualifiers, and multiple levels.

Unsupported PuzzleScript features are intentionally left out of this fixture:
properties, synonyms, aggregates, event-based sounds such as object movement sounds,
metadata commands, rule groups, rigid rules, random rules, checkpoints, messages,
and RHS `moving` propagation.
