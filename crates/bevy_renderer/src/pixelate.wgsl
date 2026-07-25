#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct PixelateSettings {
    // Physical viewport origin and size inside the render target.
    viewport: vec4<f32>,
    // Downscale factor and smoothing selector.
    parameters: vec4<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: PixelateSettings;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let target_size = vec2<f32>(textureDimensions(source_texture));
    let viewport_origin = settings.viewport.xy;
    let viewport_size = settings.viewport.zw;
    let scale = max(settings.parameters.x, 1.0);
    let target_pixel = min(in.uv * target_size, target_size - vec2(0.5));

    // ViewTarget ping-pongs between two textures. Copy pixels outside this
    // camera's viewport so applying one keyed view cannot erase sibling views.
    let viewport_max = viewport_origin + viewport_size;
    let inside_viewport =
        all(target_pixel >= viewport_origin) && all(target_pixel < viewport_max);
    let unchanged = textureLoad(source_texture, vec2<i32>(floor(target_pixel)), 0);

    // Downscale to ceil(viewport / scale), then enlarge that lattice with
    // nearest-neighbor sampling. The ceil keeps the full authored viewport
    // represented when its size is not divisible by scale.
    let local_pixel = clamp(
        target_pixel - viewport_origin,
        vec2(0.0),
        viewport_size - vec2(0.5),
    );
    let low_size = ceil(viewport_size / scale);
    let low_pixel = min(
        floor(local_pixel * low_size / viewport_size),
        low_size - vec2(1.0),
    );
    let sample_pixel = viewport_origin + (low_pixel + vec2(0.5)) * viewport_size / low_size;
    let smooth = textureSample(source_texture, source_sampler, sample_pixel / target_size);
    let nearest = textureLoad(source_texture, vec2<i32>(floor(sample_pixel)), 0);
    let pixelated = select(nearest, smooth, settings.parameters.y >= 0.5);
    return select(unchanged, pixelated, inside_viewport);
}
