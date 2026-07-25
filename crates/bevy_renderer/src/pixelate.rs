use bevy::{
    asset::{load_internal_asset, uuid_handle},
    core_pipeline::{Core3dSystems, FullscreenShader, schedule::Core3d},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FilterMode, FragmentState,
            LoadOp, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureFormat, TextureSampleType, TextureViewId,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::ViewTarget,
    },
    shader::Shader,
};

const PIXELATE_SHADER_HANDLE: Handle<Shader> = uuid_handle!("9c84e3d3-81e1-4aba-8bf6-55d91b02e439");

pub(super) struct PuzzlePixelatePlugin;

impl Plugin for PuzzlePixelatePlugin {
    fn build(&self, app: &mut App) {
        // Pure ECS owner tests intentionally install no render sub-app. The
        // typed camera component remains observable there, while GPU pipeline
        // resources only belong to apps that actually own a RenderApp.
        if app.get_sub_app(RenderApp).is_none() {
            return;
        }
        load_internal_asset!(
            app,
            PIXELATE_SHADER_HANDLE,
            "pixelate.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins((
            ExtractComponentPlugin::<PuzzlePixelatePostProcess>::default(),
            UniformComponentPlugin::<PuzzlePixelatePostProcess>::default(),
        ));

        let render_app = app
            .get_sub_app_mut(RenderApp)
            .expect("RenderApp existence was checked before installing pixelate resources");
        render_app
            .add_systems(RenderStartup, init_pixelate_pipeline)
            .add_systems(
                Core3d,
                pixelate_post_process.in_set(Core3dSystems::PostProcess),
            );
    }
}

#[derive(Component, Clone, Copy, Debug, ExtractComponent, ShaderType)]
pub(super) struct PuzzlePixelatePostProcess {
    /// x/y are the physical viewport origin; z/w are its physical dimensions.
    pub(super) viewport: Vec4,
    /// x is the authored downscale factor; y selects smooth downsampling.
    pub(super) parameters: Vec4,
}

impl PuzzlePixelatePostProcess {
    pub(super) fn new(position: UVec2, size: UVec2, scale: u16, smoothing: bool) -> Self {
        Self {
            viewport: Vec4::new(
                position.x as f32,
                position.y as f32,
                size.x as f32,
                size.y as f32,
            ),
            parameters: Vec4::new(
                f32::from(scale),
                if smoothing { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ),
        }
    }
}

#[derive(Resource)]
struct PixelatePipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline: CachedRenderPipelineId,
}

fn init_pixelate_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "puzzle_pixelate_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<PuzzlePixelatePostProcess>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("puzzle_pixelate_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: PIXELATE_SHADER_HANDLE,
            targets: vec![Some(ColorTargetState {
                // PuzzleBevy3dPlugin cameras are SDR. Bevy's SDR main-pass
                // texture is linearized through this sRGB target format.
                format: TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    commands.insert_resource(PixelatePipeline {
        layout,
        sampler,
        pipeline,
    });
}

#[derive(Default)]
struct PixelateBindGroupCache {
    cached: Option<(TextureViewId, BindGroup)>,
}

fn pixelate_post_process(
    view: ViewQuery<(
        &ViewTarget,
        &PuzzlePixelatePostProcess,
        &DynamicUniformIndex<PuzzlePixelatePostProcess>,
    )>,
    pipeline: Option<Res<PixelatePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    settings_uniforms: Res<ComponentUniforms<PuzzlePixelatePostProcess>>,
    mut cache: Local<PixelateBindGroupCache>,
    mut context: RenderContext,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let (view_target, _settings, settings_index) = view.into_inner();
    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline) else {
        return;
    };
    let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
        return;
    };
    let post_process = view_target.post_process_write();
    let bind_group = match &mut cache.cached {
        Some((texture_id, bind_group)) if *texture_id == post_process.source.id() => bind_group,
        cached => {
            let bind_group = context.render_device().create_bind_group(
                "puzzle_pixelate_bind_group",
                &pipeline_cache.get_bind_group_layout(&pipeline.layout),
                &BindGroupEntries::sequential((
                    post_process.source,
                    &pipeline.sampler,
                    settings_binding.clone(),
                )),
            );
            let (_, bind_group) = cached.insert((post_process.source.id(), bind_group));
            bind_group
        }
    };

    let mut render_pass = context
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("puzzle_pixelate_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    ..default()
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    render_pass.set_pipeline(render_pipeline);
    render_pass.set_bind_group(0, bind_group, &[settings_index.index()]);
    render_pass.draw(0..3, 0..1);
}
