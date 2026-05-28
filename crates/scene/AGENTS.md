# Agent Notes

This crate owns shared scene/presentation metadata and layout/component
contracts.

## Boundaries

Scene is shared presentation and flow metadata, not a 2D or 3D model owner.
Scene layout should place typed components and content slots; model/component
owners define behavior.

`view` is the scene root layout block. Layout primitives are `row`, `column`,
and `box`; direct `view` children behave like an implicit vertical column.

Shared embedded content should preserve the author-facing component kind, such
as `puzzle`, `puzzle3`, or `frame`, behind a model-window/component contract.
Avoid hardcoding every component keyword into the scene structural parser.

Scene navigation words are state-oriented. Prefer canonical navigation/effect
forms over legacy aliases when adding examples or tests.
