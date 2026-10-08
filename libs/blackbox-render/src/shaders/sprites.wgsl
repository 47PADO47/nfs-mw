// Textured billboards (flames, sparks, smoke): texture x vertex colour, alpha-blended or additive.
// Additive sprites fade out with the fog instead of turning towards its colour.

override ADDITIVE: bool = false;

struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
    fog_range: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>) -> VsOut {
    var out: VsOut;
    out.clip = globals.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    out.uv = uv;
    out.world = position;
    return out;
}

@fragment
fn fs_sprite(in: VsOut) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    if ADDITIVE {
        return vec4<f32>(texel.rgb, texel.a * (1.0 - fog));
    }
    return vec4<f32>(mix(texel.rgb, globals.fog_color.rgb, fog), texel.a);
}
