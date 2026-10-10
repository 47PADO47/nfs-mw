// The world material: diffuse texture x vertex colour, pre-lit or lit by one directional light, linear
// distance fog. It mirrors the native renderer's scene.wgsl.
//
// Colour: the native renderer shades and blends gamma-encoded values straight into an 8-bit UNORM surface.
// Bevy renders into an sRGB target and encodes on write. So everything here is computed in the games' gamma
// space (textures are sampled as plain UNORM, the vertex colour is the raw byte value) and the result is
// converted to linear at the very end; the hardware's sRGB encode then writes the bytes the native shader
// would have written.

import bevy_pbr::render::{
    mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_normal_local_to_world},
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
};

struct Params {
    // rgb: the fog colour (also the clear colour), in gamma space.
    fog_color: vec4<f32>,
    // x = fog start, y = fog end (f32::MAX for no fog).
    fog_range: vec4<f32>,
    // xyz: the direction the light travels (Bevy axes).
    light_dir: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var diffuse: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var diffuse_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    // D3DCOLOR bytes: B, G, R, A.
    @location(3) color_bgra: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    let model = get_world_from_local(v.instance_index);
    let world = mesh_position_local_to_world(model, vec4<f32>(v.position, 1.0));
    var out: VertexOutput;
    out.clip = position_world_to_clip(world.xyz);
    out.world = world.xyz;
    out.normal = mesh_normal_local_to_world(v.normal, v.instance_index);
    out.uv = v.uv;
    out.color = v.color_bgra.zyxw;
    return out;
}

// The exact sRGB decode, so decode then the hardware's encode is the identity.
fn linear_from_gamma(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn shade(in: VertexOutput) -> vec4<f32> {
    // The camera's mip bias carries the native renderer's texture LOD bias.
    let base = textureSampleBias(diffuse, diffuse_sampler, in.uv, view.mip_bias) * in.color;
    var lit: vec3<f32>;
    @if(PRELIT) {
        // Pre-lit geometry: vertex colours use 0x80 for full brightness.
        lit = min(base.rgb * 2.0, vec3<f32>(1.0));
    }
    @else {
        let n = normalize(in.normal);
        let diffuse_light = max(dot(n, -params.light_dir.xyz), 0.0);
        // Thin parts are often seen from behind: give back faces a little light.
        let back_light = max(dot(-n, -params.light_dir.xyz), 0.0) * 0.25;
        lit = base.rgb * (0.35 + 0.65 * diffuse_light + back_light);
    }
    var fog = 0.0;
    @if(FOG) {
        let distance = length(in.world - view.world_position);
        fog = clamp((distance - params.fog_range.x) / max(params.fog_range.y - params.fog_range.x, 0.001), 0.0, 1.0);
    }
    return vec4<f32>(mix(lit, params.fog_color.rgb, fog), base.a);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = shade(in);
    @if(ALPHA_TEST) {
        if c.a < 0.5 {
            discard;
        }
    }
    @if(OPAQUE_ALPHA) {
        return vec4<f32>(linear_from_gamma(c.rgb), 1.0);
    }
    @else {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    }
}
