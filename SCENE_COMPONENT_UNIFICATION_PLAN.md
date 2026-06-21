# Scene And Component Unification Plan

This plan separates the shared scene/component surface from the parts that are
truly model-specific. It is intended for parallel forked sessions, so each work
package names its owner boundary, dependencies, and validation target.

## Problem

The current codebase has started moving scene layout into shared ownership via
`puzzle-scene`, but scene/component handling is still split across the 2D
language model and the 3D model parser/runtime.

The visible symptoms are:

- `view`, `row`, `column`, and `box` layout attributes are shared, but their
  containing scene/component ASTs are separate.
- 2D and 3D parsers both walk scene blocks, view blocks, and nested container
  children.
- Component keyword lists and block-depth logic are repeated in authoring
  collection, parsing, export, and runtime rendering.
- 2D and 3D JSON export paths encode the same layout object independently.
- 2D scene syntax has moved to `inputs { ... }`, while 3D scene syntax still has
  compatibility paths such as `keys { ... }`.

The deeper issue is not only duplicate code. The implementation still implies
that "2D scene" and "3D scene" are different concepts. The canonical model says
they should be one scene/component layer with model-specific components inside
it.

## Principle

Scene is a presentation and flow owner, not a 2D or 3D model owner.

Shared scene concepts must be represented once:

- scene identity
- root layout
- scene resources and state declarations where applicable
- scene inputs
- scene transitions/effects
- component tree traversal
- generic components such as `title`, `subtitle`, `text`, `button`, `row`,
  `column`, `box`, `for`, and `level_menu`
- shared component layout metadata

Model-specific concepts stay model-specific:

- 2D puzzle model syntax, rules, levels, sprites, viewport, and renderer
- 3D puzzle model syntax, rules, levels, sprites/voxels, camera, and renderer
- model-window components such as `puzzle` versus `puzzle3`
- runtime model input interpretation and intrinsic model size

The boundary should be:

```txt
.puzzle source
  -> puzzle-lang parses shared scene/component syntax
  -> model-specific parsers parse model bodies
  -> shared scene document contains model slots/components
  -> adapters render shared scene tree and delegate model windows
```

## Target Shape

Create a shared scene module/crate that owns the stable scene/component data
model and generic parsing helpers.

The existing `puzzle-scene` crate is the natural owner. It should grow from
"layout metadata" into "shared scene surface", while still avoiding dependency
on `puzzle-core`, `puzzle-lang`, `puzzle-play`, HTML, or model-specific crates.

Target shared types:

```rust
pub struct Scene {
    pub name: String,
    pub layout: SceneLayout,
    pub components: Vec<SceneComponent>,
    pub inputs: Vec<SceneInputBinding>,
    pub transitions: Vec<SceneTransition>,
}

pub enum SceneComponent {
    ModelWindow(ModelWindowComponent),
    Title(SceneTextExpr),
    Subtitle(SceneTextExpr),
    Text(SceneTextExpr),
    Button(SceneButton),
    Row(SceneContainer),
    Column(SceneContainer),
    Box(SceneContainer),
    For(SceneFor),
    LevelMenu(LevelMenuComponent),
    Menu(MenuInstance),
}

pub struct ModelWindowComponent {
    pub model_kind: ModelKind,
    pub source: String,
    pub layout: SceneLayout,
}

pub enum ModelKind {
    Puzzle2d,
    Puzzle3d,
}
```

This sketch is not final API. The key point is that the tree shape and generic
component semantics are shared; the model window is the extension point.

## What Must Stay Separate

Do not move these into the shared scene crate:

- 2D rule parsing and lowering into `puzzle-core`
- 3D rule parsing and model state types
- level parsing details other than shared resource references
- renderer implementation and DOM/canvas layout mechanics
- runtime session behavior such as undo/restart/level advance
- theme/sounds/asset adapter behavior unless it is already represented as a
  shared scene effect payload

Do not solve component behavior by adding global scene shortcuts. For example,
`level_menu` may own cursor and enter behavior, but `for level in levels` must
remain a generic data loop.

## Work Packages

### Package A: Inventory And Canonical Contract

Owner: documentation and tests.

Goal: make the intended boundary explicit before moving code.

Tasks:

- Add canonical examples for mixed 2D/3D scenes using the same `scene`,
  `view`, `row`, `column`, and `box` forms.
- Document that `view size 4 3 { ... }` is root scene logical layout, independent
  of whether the model window is 2D or 3D.
- List canonical component keywords in one developer-facing place.
- Mark legacy 3D `keys { ... }` as compatibility or schedule its removal in
  favor of `inputs { ... }`.

Validation:

- No runtime behavior change required.
- Docs should give forked sessions a single target syntax to preserve.

### Package B: Shared Scene AST

Owner: `crates/scene`.

Status: baseline implemented. `puzzle-scene` now defines shared scene/component
types, model-window leaves, component kind keyword mapping, and traversal/layout
helpers. Existing 2D/3D parsers have not been migrated yet.

Goal: define the shared data model without changing parsers yet.

Tasks:

- Move or mirror shared component structs into `puzzle-scene`.
- Keep model-specific payloads generic enough for both current callers:
  `ModelWindow { model_kind, source, layout }`.
- Add helper methods:
  - `component.kind()`
  - `component.children()`
  - `component.children_mut()`
  - `component.layout()`
  - `component.layout_mut()`
- Add `SceneComponentKind` constants or enum to remove repeated keyword lists.

Validation:

- `cargo test -p puzzle-scene`
- No downstream migration required in this package.

### Package C: Shared Component Parser Kernel

Owner: parser infrastructure.

Goal: share traversal and container parsing while leaving leaf parsing
model-aware.

Status: implemented. `puzzle-scene` now owns `SceneBlockSyntax`,
`parse_scene_layout_header`, `parse_scene_component_block`, and
`parse_scene_component_at`. The 2D and 3D parsers still keep their own leaf
component parsing, but both delegate `view` layout headers and nested
`row` / `column` / `box` traversal to the shared kernel.

Tasks:

- Add a generic parser helper for:
  - block header validation
  - `view` layout parsing
  - `row` / `column` / `box` block parsing
  - nested component traversal
  - block missing-end errors
- Let callers provide a leaf parser callback for model-specific components:
  - 2D accepts `puzzle <slot>`, `text`, `subtitle`, `for`, `menu`, etc.
  - 3D accepts `puzzle3 <slot>` and any 3D-only compatibility leaf while it
    exists.
- Centralize generic component keyword detection so authoring collection and
  parser traversal use the same source of truth.

Validation:

- Existing focused 2D scene tests still pass.
- Existing 3D scene parser tests still pass.
- Add at least one shared parser test for nested `row` / `column` / `box`
  layout attributes that is reused by both callers or runs in `puzzle-scene`.

### Package D: Migrate 2D SceneDef To Shared Shape

Owner: `puzzle-lang`.

Status: implemented for the 2D component tree. `puzzle-lang` keeps its
2D-specific scene effects, scene expressions, resources, state, and puzzle rule
metadata, but `SceneComponent` and child containers now use the shared
`puzzle-scene` shape. 2D puzzle slots lower to `ModelWindow` with
`ModelKind::Puzzle2d` instead of a 2D-only `PuzzleState` variant.

Goal: make 2D scenes consume the shared AST without changing canonical syntax.

Tasks:

- Replace or alias `SceneDef` / `SceneComponent` fields to shared scene types
  where possible.
- Keep 2D-only fields outside the shared component tree, or wrap them in shared
  scene extensions only when they are truly scene-level.
- Preserve existing scene transition/effect parsing unless Package E is ready.
- Update tests to assert shared shape, not 2D-only constructors.

Validation:

- `cargo test -p puzzle-lang` focused scene tests.
- `cargo check -p html-play`.

### Package E: Migrate 3D Scenes To Shared Shape

Owner: `puzzle-3d` facade, with deterministic 3D state owned by `puzzle-grid3d`.

Goal: stop representing 3D scenes as a separate scene/component hierarchy.

Tasks:

- Replace the former 3D-only scene wrapper with `puzzle_scene::Scene`.
- Replace `SceneComponent3` with shared components; `puzzle3 <slot>` lowers to a
  shared `FrameComponent` whose `inputs` are component-local window config.
- Convert `puzzle3 <slot>` into shared `ModelWindow { model_kind: Puzzle3d }`.
- Move scene root layout to the shared `Scene`.
- Decide whether 3D `keys { ... }` is immediately migrated to `inputs { ... }`
  or kept as a compatibility parser that lowers into shared scene inputs.

Validation:

- `cargo test -p puzzle-3d`
- `cargo test -p puzzle-grid3d`
- Fixture export should still include the same scene JSON shape.

### Package F: Shared Scene JSON

Owner: export/adapters.

Goal: remove duplicate layout/component JSON encoding.

Tasks:

- Add a shared JSON or serde representation for `SceneLayout`.
- Prefer deriving serde for shared scene/component export if dependency policy
  allows it.
- If serde is not acceptable, put one string writer in `puzzle-scene` and call
  it from both 2D HTML export and 3D fixture export.
- Normalize output field names:
  - `kind`
  - `layout`
  - `children`
  - `source`
  - `modelKind` or equivalent for model windows

Validation:

- `cargo check -p html-play`
- `cargo test -p puzzle-3d`
- `cargo test -p puzzle-grid3d`
- Diff generated fixture JSON intentionally if field names change.

### Package G: Runtime Rendering Boundary

Owner: HTML runtimes.

Goal: render one shared scene tree and delegate model windows.

Tasks:

- Identify the minimum shared JavaScript scene renderer behavior:
  - traverse component tree
  - apply `row` / `column` / `box` layout
  - apply root scene layout
  - render generic title/text/button/level_menu
  - delegate `ModelWindow(Puzzle2d)` or `ModelWindow(Puzzle3d)`
- Decide whether this becomes a shared JS module or whether both runtimes call a
  generated shared JSON contract first.
- Remove copied layout math only after the shared contract is stable.

Validation:

- Browser or Playwright check for a 2D scene with `view size 4 3`.
- Browser or Playwright check for a 3D scene with the same layout shape.
- Component embed mode must remain a model-window contract; it must not cause a
  child scene to reinterpret its own scene layout inside the host component.

## Parallelization Strategy

Safe to run in parallel:

- Package A docs/tests inventory.
- Package B shared AST additions, as long as it does not force downstream
  migration yet.
- Package F layout JSON helper extraction, if it only targets `SceneLayout`.

Run after Package B:

- Package C shared parser kernel.

Run after Package C:

- Package D 2D migration.
- Package E 3D migration.

Run after Package D and E have a stable JSON contract:

- Package G runtime rendering unification.

Avoid running Package D and E in parallel against the same shared API unless
Package B has landed first. Otherwise both branches will invent slightly
different extension points.

## Acceptance Criteria

The unification is complete when:

- There is one shared scene/component tree type for generic scene structure.
- `view`, `row`, `column`, and `box` are parsed through one shared path.
- Generic component keyword detection exists in one place.
- 2D and 3D model windows are leaves in the same component tree.
- Scene layout JSON is emitted by one shared helper or serde shape.
- 2D and 3D canonical examples use the same scene syntax.
- Any remaining 2D/3D difference can be explained as model syntax, model
  runtime, or model renderer behavior.

## Current Known DRY Violations To Track

- `SceneComponent` in `crates/lang/src/loaded.rs` and `SceneComponent3` in
  `crates/puzzle_3d/src/scene.rs`.
- 2D scene parsing around `parse_screen_view_block` / `parse_screen_component`
  and 3D scene parsing around `parse_scene_block` / `parse_scene_component_at`.
- Component keyword lists in authoring collection, block parsing, and component
  parsing.
- Layout JSON writing in `html-play` and `puzzle_3d` fixture export.
- Copied 3D visual runtime code between `crates/html_play/static/puzzle3_app.js`
  and `tests/puzzle_3d/visual/app.js`.
- Canonical drift between 2D `inputs { ... }` and 3D `keys { ... }`.

## Non-Goals

- Do not make 2D and 3D puzzle models share rule/state internals.
- Do not introduce a generic game engine scene graph with arbitrary transforms.
- Do not move rendering into `puzzle-core`.
- Do not widen canonical syntax just to preserve old tests.
- Do not make `for level in levels` behave like `level_menu`.

## Suggested First Forks

1. Shared AST fork: implement Package B only, with tests in `puzzle-scene`.
2. Parser kernel fork: prototype Package C against the current AST, behind
   adapter functions if needed.
3. Syntax cleanup fork: finish canonicalizing old `keys` tests and document the
   3D migration path to `inputs`.
4. Export fork: extract `SceneLayout` JSON writing first, then wait for the
   shared component AST before touching full component JSON.
