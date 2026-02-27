struct Params {
    image_size: vec2<f32>,
    window_size: vec2<f32>,
    pan: vec2<f32>,
    zoom: f32,
    is_grid_item: f32,
    is_selected: f32,
    _pad: f32,
    _pad2: vec2<f32>, // Pad to 48 bytes (12 floats)
};

@group(1) @binding(0)
var<uniform> params: Params;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) quad_uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    let pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    let p = pos[vertex_index];
    
    let base_uv = p * 0.5 + 0.5;
    out.quad_uv = base_uv;

    if (params.is_grid_item > 0.5) {
        // Grid Mode: params.pan is [x, y] in pixels, params.zoom is box size in pixels
        var quad_size = vec2<f32>(params.zoom, params.zoom);
        if (params._pad2.x > 0.0) {
            quad_size.y = params._pad2.x;
        }
        let pixel_pos = params.pan + base_uv * quad_size;
        
        let clip_x = (pixel_pos.x / params.window_size.x) * 2.0 - 1.0;
        let clip_y = 1.0 - (pixel_pos.y / params.window_size.y) * 2.0;
        out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
        
        let aspect = params.image_size.x / params.image_size.y;
        var uv = base_uv;
        if (aspect > 1.0) {
            uv.x = (uv.x - 0.5) * (params.image_size.y / params.image_size.x) + 0.5;
        } else if (aspect < 1.0) {
            uv.y = (uv.y - 0.5) * (params.image_size.x / params.image_size.y) + 0.5;
        }
        out.uv = uv; // Quad is already flipped in clip_y calculation
    } else {
        // Single View Mode: ABSOLUTE SCALING (1.0 = 1:1 pixel)
        let base_scale = params.image_size / params.window_size;
        let final_scale = base_scale * params.zoom;
        
        let pixel_pan = params.pan / params.window_size * 2.0;
        let clip_pos = p * final_scale; 
        
        out.clip_position = vec4<f32>(clip_pos.x + pixel_pan.x, clip_pos.y - pixel_pan.y, 0.0, 1.0);
        out.uv = vec2<f32>(base_uv.x, 1.0 - base_uv.y); // Flip Y to match image crate
    }
    
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (params.is_grid_item > 0.5) {
        if (in.quad_uv.x < 0.0 || in.quad_uv.x > 1.0 || in.quad_uv.y < 0.0 || in.quad_uv.y > 1.0) {
            discard;
        }
    }

    if (in.uv.x < 0.0 || in.uv.x > 1.0 || in.uv.y < 0.0 || in.uv.y > 1.0) {
        if (params.is_grid_item > 0.5) {
            return vec4<f32>(0.05, 0.05, 0.06, 1.0);
        }
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    
    var color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    if (params.is_grid_item > 0.5 && params.is_selected > 0.5) {
        let border = 2.0 / params.zoom;
        if (in.quad_uv.x < border || in.quad_uv.x > (1.0 - border) || in.quad_uv.y < border || in.quad_uv.y > (1.0 - border)) {
            return vec4<f32>(1.0, 0.8, 0.1, 1.0); // Vibrant orange for selection
        }
    }
    
    return color;
}