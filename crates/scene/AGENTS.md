# Agent Notes

This crate owns shared scene/presentation metadata and layout/component
contracts.

## Boundaries

Scene is shared presentation and flow metadata, not a 2D or 3D model owner.
Scene layout should place typed components and content slots; model/component
owners define behavior.

`layout` is the scene root layout block. Layout primitives are `row`, `column`,
and `box`; direct `layout` children behave like an implicit vertical column.

Shared embedded content should preserve the author-facing component kind, such
as `puzzle`, `puzzle3`, or `frame`, behind a model-window/component contract.
Avoid hardcoding every component keyword into the scene structural parser.

Scene navigation words are state-oriented. Prefer canonical navigation/effect
forms over legacy aliases when adding examples or tests.

Text presentation is one component contract with `heading`, `subheading`,
`body`, and `caption` roles. Metadata names such as `title` and `subtitle` are
content values, not component kinds. Space allocation (`fit` / weighted `fill`),
aspect ratio, cross-axis alignment, and main-axis distribution are scene
semantics; adapters only project these typed values into their host layout.
