// Shadow depth for the custom material. Uses the same alpha cutoff as the colour pass.
import bevy_pbr::render::{
    mesh_functions::{get_world_from_local, mesh_position_local_to_world},
    view_transformations::position_world_to_clip,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(1) var diffuse: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var diffuse_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct Output {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    @location(2) unclipped_depth: f32,
};

@vertex
fn vertex(v: Vertex) -> Output {
    let world = mesh_position_local_to_world(get_world_from_local(v.instance_index), vec4(v.position, 1.0));
    var out: Output;
    out.clip = position_world_to_clip(world.xyz);
    out.uv = v.uv;
    out.alpha = v.color.a;
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
        out.unclipped_depth = out.clip.z;
        out.clip.z = min(out.clip.z, 1.0);
    }
    return out;
}

fn alpha_test(in: Output) {
    @if(ALPHA_TEST) {
        var alpha = textureSample(diffuse, diffuse_sampler, in.uv).a;
        @if(!GLOSSY) {
            alpha *= in.alpha;
        }
        if alpha < 0.5 {
            discard;
        }
    }
}

@if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
@fragment
fn fragment(in: Output) -> @builtin(frag_depth) f32 {
    alpha_test(in);
    return in.unclipped_depth;
}
@else
@fragment
fn fragment(in: Output) {
    alpha_test(in);
}
