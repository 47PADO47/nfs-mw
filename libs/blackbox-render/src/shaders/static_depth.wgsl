// The static scene's depth, copied in the prepass before the dynamic models are drawn, and the screen tiles in
// which a dynamic model covers it.
@group(0) @binding(0) var depth: texture_depth_2d;
@group(0) @binding(1) var copy: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8)
fn snapshot(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(copy)) { return; }
    textureStore(copy, id.xy, vec4<f32>(textureLoad(depth, vec2<i32>(id.xy), 0)));
}

// One 16-by-16 pixel tile per workgroup of 8x8 (the portable limit), each invocation testing 2x2 pixels: 1 where a
// dynamic model covers any of them. Tiles past the image edge (the tile image has power-of-two sides) are 0.
@group(0) @binding(2) var static_depth: texture_2d<f32>;
@group(0) @binding(3) var tiles: texture_storage_2d<r32float, write>;
var<workgroup> covered: atomic<u32>;

@compute @workgroup_size(8, 8)
fn cover(@builtin(local_invocation_id) local: vec3<u32>, @builtin(workgroup_id) group: vec3<u32>) {
    if all(local.xy == vec2<u32>(0u)) { atomicStore(&covered, 0u); }
    workgroupBarrier();
    let size = textureDimensions(depth);
    for (var i = 0u; i < 4u; i++) {
        let p = group.xy * 16u + local.xy * 2u + vec2<u32>(i & 1u, i >> 1u);
        if all(p < size) && textureLoad(static_depth, p, 0).r != textureLoad(depth, p, 0) {
            atomicOr(&covered, 1u);
        }
    }
    workgroupBarrier();
    if all(local.xy == vec2<u32>(0u)) { textureStore(tiles, group.xy, vec4<f32>(f32(atomicLoad(&covered)))); }
}

// The next coarser tile level: the maximum of each 2-by-2 block.
@group(0) @binding(4) var finer: texture_2d<f32>;
@group(0) @binding(5) var coarser: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8)
fn reduce(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(coarser)) { return; }
    // A side of one texel has no second texel to fold in.
    let last = textureDimensions(finer) - 1u;
    let base = id.xy * 2u;
    let far = min(base + 1u, last);
    let value = max(max(textureLoad(finer, base, 0).r, textureLoad(finer, vec2<u32>(far.x, base.y), 0).r),
                    max(textureLoad(finer, vec2<u32>(base.x, far.y), 0).r, textureLoad(finer, far, 0).r));
    textureStore(coarser, id.xy, vec4<f32>(value));
}
