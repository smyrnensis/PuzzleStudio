use bevy::prelude::*;
use puzzle_bevy_renderer::{
    PuzzleBevy3dPlugin, PuzzleBevy3dRenderSettings, PuzzleBevy3dView, PuzzleBevyCamera,
    PuzzleBevyFramebufferRect, PuzzleBevyLighting, PuzzleBevyViewId, submit_resolved_frame,
};
use puzzle_runtime_contract::{
    RuntimeLinearRgba, RuntimeResolvedRenderBatch, RuntimeResolvedRenderBatchContent,
    RuntimeResolvedRenderFrame, RuntimeResolvedVoxel,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "PuzzleStudio Bevy renderer".to_string(),
                canvas: Some("#puzzle-bevy".to_string()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PuzzleBevy3dPlugin::default())
        .add_systems(Startup, submit_example_frame)
        .run();
}

fn submit_example_frame(world: &mut World) {
    let frame = RuntimeResolvedRenderFrame {
        batches: vec![RuntimeResolvedRenderBatch {
            identity: puzzle_runtime_contract::RuntimeResolvedRenderBatchIdentity {
                render_order: 0,
                object_ids: vec![1],
                visual_ids: vec!["example".to_string()],
                instance_ids: vec![1],
                cell: [0, 0, 0],
            },
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            opacity: 1.0,
            pixel_geometry: None,
            content: RuntimeResolvedRenderBatchContent::Voxels {
                width: 2,
                depth: 2,
                height: 2,
                voxels: [
                    ([0, 0, 0], [1.0, 0.15, 0.08, 1.0]),
                    ([1, 0, 0], [0.1, 0.45, 1.0, 1.0]),
                    ([0, 1, 0], [0.2, 0.9, 0.4, 1.0]),
                    ([0, 0, 1], [1.0, 0.8, 0.15, 1.0]),
                ]
                .into_iter()
                .map(
                    |(position, [red, green, blue, alpha])| RuntimeResolvedVoxel {
                        position,
                        color: RuntimeLinearRgba {
                            red,
                            green,
                            blue,
                            alpha,
                        },
                    },
                )
                .collect(),
            },
        }],
        decorations: Vec::new(),
        next_sample: None,
    };
    submit_resolved_frame(
        world,
        PuzzleBevyViewId::three_d("example", "main"),
        PuzzleBevy3dView {
            active: true,
            order: 0,
            framebuffer: PuzzleBevyFramebufferRect {
                physical_position: UVec2::ZERO,
                physical_size: UVec2::new(800, 600),
            },
            clear_color: Color::linear_rgb(0.025, 0.03, 0.045),
            camera: PuzzleBevyCamera::default(),
            lighting: PuzzleBevyLighting {
                intensity: 1.0,
                ambient: 1.0,
                yaw_degrees: 53.0,
                pitch_degrees: 56.0,
                color: Color::WHITE,
            },
            shadows_enabled: true,
            render_settings: PuzzleBevy3dRenderSettings::default(),
        },
        &frame,
    )
    .expect("example frame must satisfy the Bevy contract");
}
