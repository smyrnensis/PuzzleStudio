# Agent Notes

This crate owns renderer-neutral visual planning shared by every presentation
backend.

## Boundaries

- Resolve compiled visual priorities and composition modes before an adapter
  builds a display list or GPU scene.
- Resolve animation occurrence channels and prepare compatible visual tween
  channels before an adapter samples presentation time.
- Resolve authored static transform sequences into canonical affine matrices;
  adapters may apply those matrices in their host coordinate system but must
  not reconstruct static transform order from author operations.
- Normalize palette-backed visual rows into sparse linear-RGBA pixels or
  voxels, select clip frames from an explicit elapsed time, and execute owned
  composition modes before a backend creates draw resources.
- Keep algorithms deterministic and source-free. Consume typed compiled/runtime
  contracts; never parse `.puzzle` text or inspect serialized JSON.
- Do not own Canvas, DOM, Three.js, Bevy, GPU resources, rasterization, clocks,
  camera matrices, face culling, mesh generation, batching, or asset IO.
- A renderer may sample a prepared numeric tween channel and map resolved
  pixels, voxels, external images, and affine matrices to backend resources.
  It must not infer priority, merge policy, palette colors, clip timing,
  transform compatibility, shortest-angle direction, or occurrence conflict
  resolution.

## Tests

```bash
cargo test -p puzzle-presentation
```
