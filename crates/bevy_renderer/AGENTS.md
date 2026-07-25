# Agent Notes

This crate owns the Bevy implementation of the renderer backend.

## Boundaries

- Consume only `RuntimeResolvedRenderFrame`; do not resolve visual names,
  palettes, frame timing, animation channels, priority, or composition.
- Own Bevy ECS entities, meshes, materials, cameras, lights, shadows, culling,
  batching compatibility, and GPU-facing resource lifetime.
- Reject unsupported resolved primitives explicitly. Do not inspect legacy
  Puzzle3 visual resources or serialized authoring data as a fallback.
- Keep the pure frame-to-instance projection testable without a GPU or window.

## Commands

```bash
cargo test -p puzzle-bevy-renderer
cargo check -p puzzle-bevy-renderer --example resolved_voxels
cargo check -p puzzle-bevy-renderer --target wasm32-unknown-unknown
```
