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
- `WINCONDITIONS` rows are lowered into canonical condition expressions; PuzzleScript `all Target on Crate` becomes `no [ Target no Crate ]`.
- Empty PS `WINCONDITIONS` sections are omitted from canonical output.
- `RULES` rows are copied as canonical rules. Prefixless rules containing PS movement markers (`>`, `<`, `^`, `v`) rely on canonical implicit cardinal expansion.
- PS's special `Player` movement is represented by inserting `input directions [ Player ] -> [ > Player ]`.
- PS movement markers use the canonical anonymous movement mark, and the existing built-in `move` routine resolves the movement phase.
- PS `again` rule suffixes become canonical `again` effects. Runtime-owned
  automatic no-input follow-up turns handle the repeat; the importer does not
  synthesize `__ps_again` state or duplicate movement guards.
- PS `late` rules are emitted after `move`.
- Imported level clear handlers emit `wait 0.3s` before `next_level`.
- PS `moving` / `stationary` qualifiers become anonymous movement mark predicates such as `directions Crate` and `Crate{no directions}` on LHS; RHS `stationary` is emitted as the bare object.
- Simple PS `SOUNDS` rows such as `sfx0 12345` become canonical `sounds { sfx sfx0 seed=12345 type=puzzlescript }`; PS rule suffixes such as `SFX0` become `sfx sfx0`.
- `LEVELS` splits blank-line-separated PS levels into canonical unnamed levels.
- Default `scene title` and `scene playing` entries are generated. Title
  choices call scene-local routines directly, while the playing scene mounts the
  imported `main` puzzle as `puzzle board = main` and steps `board`.
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
