struct Params {
    image_size: vec2<f32>,
    window_size: vec2<f32>,
    pan: vec2<f32>,
    zoom: f32,
    is_grid_item: f32,
    is_selected: f32,
    _pad: f32,
    _pad2: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> params: Params;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Full quad vertices
    let pos = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0)
    );
    let p = pos[vertex_index];
    
    let uv = (p + 1.0) * 0.5;
    out.uv = uv;

    // Calculate quad size
    var quad_size = vec2<f32>(params.zoom, params.zoom);
    if (params._pad2.x > 0.0) {
        quad_size.y = params._pad2.x;
    }
    
    // Calculate pixel position (top-left corner of quad)
    let pixel_pos = params.pan + uv * quad_size;
    
    // Convert to clip space
    let clip_x = (pixel_pos.x / params.window_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (pixel_pos.y / params.window_size.y) * 2.0;
    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, in.uv);
    return color;
}