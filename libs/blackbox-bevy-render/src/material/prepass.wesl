// Prepass for the world material: depth for shadow maps and for the screen camera's depth prepass, and motion
// vectors for the temporal upscalers (FSR 3, DLSS) and TAA. One shader serves both, told apart by the shader defs
// Bevy adds: a light's shadow view has only DEPTH_PREPASS, a camera with `MotionVectorPrepass` also has
// MOTION_VECTOR_PREPASS. Alpha-tested draws discard here the way the colour pass does (ALPHA_TEST, see `prepass.rs`).
// Blended and additive draws never reach a prepass; Bevy queues only opaque and alpha-masked materials.

import bevy_pbr::render::{
    mesh_functions::{get_world_from_local, get_previous_world_from_local, mesh_position_local_to_world},
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
};
@if(MOTION_VECTOR_PREPASS)
import bevy_pbr::prepass::bindings::previous_view_uniforms;

@group(constants::MATERIAL_BIND_GROUP) @binding(1) var diffuse: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var diffuse_sampler: sampler;

// Prefix of the shared Scene storage buffer used by the colour pass.
struct Params {
    fog_color: vec4<f32>,
    fog_range: vec4<f32>,
    light_dir: vec4<f32>,
    tone_map: vec4<f32>,
    surface_ambient: vec4<f32>,
    surface_diffuse: vec4<f32>,
    scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>,
    to_sun: vec4<f32>,
    scenery: vec4<f32>,
};
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<storage, read> params: Params;

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
    @if(MOTION_VECTOR_PREPASS)
    @location(3) world: vec4<f32>,
    @if(MOTION_VECTOR_PREPASS)
    @location(4) previous_world: vec4<f32>,
};

// The same transform as the colour pass: the jittered clip position, so the depth matches the colour pass's, and
// the previous frame's model matrix for the motion vectors.
@vertex
fn vertex(v: Vertex) -> Output {
    let world = mesh_position_local_to_world(get_world_from_local(v.instance_index), vec4<f32>(v.position, 1.0));
    var out: Output;
    out.clip = position_world_to_clip(world.xyz);
    out.uv = v.uv;
    out.alpha = v.color.a;
    @if(MOTION_VECTOR_PREPASS) {
        out.world = world;
        out.previous_world = mesh_position_local_to_world(
            get_previous_world_from_local(v.instance_index),
            vec4<f32>(v.position, 1.0),
        );
    }
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
        out.unclipped_depth = out.clip.z;
        out.clip.z = min(out.clip.z, 1.0);
    }
    return out;
}

fn alpha_test(in: Output) {
    @if(ALPHA_TEST) {
        // The colour pass's texture alpha, sampled with the same mip bias, so the same texels survive.
        var alpha = textureSampleBias(diffuse, diffuse_sampler, in.uv, view.mip_bias).a;
        @if(!GLOSSY) {
            var vertex_alpha = in.alpha;
            @if(TREE) {
                if params.scenery.x > 0.5 {
                    vertex_alpha = 1.0;
                }
            }
            @if(WORLD_PRELIT) {
                if params.scenery.x > 0.5 {
                    vertex_alpha *= 2.0;
                }
            }
            alpha *= vertex_alpha;
        }
        if alpha < 0.5 {
            discard;
        }
    }
}

// Motion in Bevy's convention (`bevy_pbr`'s prepass.wesl): the clip-space offset from last frame's unjittered
// position to this frame's, halved into UV units with V flipped.
@if(MOTION_VECTOR_PREPASS)
fn motion_vector(in: Output) -> vec2<f32> {
    let clip = view.unjittered_clip_from_world * in.world;
    let previous_clip = previous_view_uniforms.unjittered_clip_from_world * in.previous_world;
    return (clip.xy / clip.w - previous_clip.xy / previous_clip.w) * vec2<f32>(0.5, -0.5);
}

@if(MOTION_VECTOR_PREPASS)
@fragment
fn fragment(in: Output) -> @location(1) vec2<f32> {
    alpha_test(in);
    return motion_vector(in);
}

@if(!MOTION_VECTOR_PREPASS && UNCLIPPED_DEPTH_ORTHO_EMULATION)
@fragment
fn fragment(in: Output) -> @builtin(frag_depth) f32 {
    alpha_test(in);
    return in.unclipped_depth;
}

@if(!MOTION_VECTOR_PREPASS && !UNCLIPPED_DEPTH_ORTHO_EMULATION)
@fragment
fn fragment(in: Output) {
    alpha_test(in);
}
