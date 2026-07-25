# PuzzleStudio Bevy renderer

This crate contains the native and WebAssembly-capable 2D and 3D renderer backends for the
typed PuzzleStudio presentation contract.

```text
authoring / runtime state
  -> puzzle-presentation
  -> RuntimeResolvedRenderFrame
  -> puzzle-bevy-renderer
  -> Bevy ECS / PBR / wgpu
```

The backend never receives visual names, palette tokens, animation channels,
priority declarations, or composition rules. `puzzle-presentation` resolves
those meanings first. `PuzzleBevy3dPlugin` accepts keyed views through
`submit_resolved_frame`, projects canonical XYZ into Bevy's Y-up coordinates,
and reconciles each view's voxel entities without replacing stable entities.

`PuzzleBevy2dPlugin` accepts the same frame type through
`submit_resolved_frame_2d`. Logical ASCII and solid pixels become vertex-colored
mesh quads and are rasterized only into the final framebuffer. Axis boundaries
are computed once per batch, so neighboring pixels share the exact same mesh
edge coordinate. The backend does not create or scale an intermediate texture
for logical pixels.

Each visible voxel uses a shared unit-cube mesh. Materials are cached by linear
RGBA, so Bevy can batch compatible mesh/material pairs and retain ownership of
culling, shadows, render phases, GPU buffers, and platform-specific wgpu work.
Transparent samples remain separate; a later opaque sample at the exact same
world transform removes earlier coincident samples before GPU submission.

## Host integration

```rust
use bevy::prelude::*;
use puzzle_bevy_renderer::{
    PuzzleBevy2dPlugin, PuzzleBevy2dView, PuzzleBevyFramebufferRect,
    PuzzleBevyViewId, submit_resolved_frame_2d,
};

let mut app = App::new();
app.add_plugins(DefaultPlugins)
    .add_plugins(PuzzleBevy2dPlugin::default());

// `frame` is produced directly by puzzle_presentation::resolve_render_moment.
// `visual_images` is the typed catalog supplied by the asset adapter and used
// during that resolution.
submit_resolved_frame_2d(
    app.world_mut(),
    PuzzleBevyViewId::two_d("board", "main"),
    PuzzleBevy2dView {
        order: 0,
        framebuffer: PuzzleBevyFramebufferRect {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(800, 600),
        },
        clear_color: Color::BLACK,
        origin: Vec2::ZERO,
        size: Vec2::new(20.0, 15.0),
        active: true,
    },
    visual_images,
    &frame,
)?;
```

Each submitted view owns a camera, framebuffer rectangle, render-layer
namespace, frame entities, and, for 3D, directional and ambient lighting.
Removing one view cannot despawn or expose another view's entities.
`PuzzleBevyCamera` supports perspective and orthographic projection.

## Explicit boundaries

- Pixel batches are rejected by the 3D backend, and voxel batches are rejected
  by the 2D backend.
- Raster/external-image batches are distinct from logical pixels. The 2D
  backend validates their asset ID, revision, and dimensions against the typed
  decoded RGBA catalog supplied by the host; a missing or stale asset fails
  explicitly. The logical-pixel path never substitutes for that contract.
- Affine shear is rejected because Bevy `Transform` cannot represent it
  exactly. Translation, rotation, reflection, and scale remain supported.
- Clocks and frame sampling stay outside this crate. The host submits each
  frame returned by `puzzle-presentation`.

## Verification

```bash
cargo test -p puzzle-bevy-renderer
cargo check -p puzzle-bevy-renderer --example resolved_voxels
cargo check -p puzzle-bevy-renderer --target wasm32-unknown-unknown
cargo run -p puzzle-bevy-renderer --example resolved_voxels
```

The example uses the DOM canvas `#puzzle-bevy` on WebAssembly and a normal
window on native platforms.
