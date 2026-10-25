// Appended to the production cascaded receiver functions. No material lighting is accumulated.
struct ResolveParams { inverse:mat4x4<f32>, control:vec4<f32> };
@group(0) @binding(0) var<uniform> resolve:ResolveParams;
@group(0) @binding(1) var resolve_depth:texture_depth_2d;
@group(0) @binding(2) var resolve_normals:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index)i:u32)->@builtin(position)vec4<f32> {
    let p=vec2(f32((i<<1u)&2u),f32(i&2u));return vec4(p*2.0-1.0,0.0,1.0);
}
@fragment fn fs(@builtin(position)pixel:vec4<f32>)->@location(0)vec4<f32> {
    let q=vec2<i32>(pixel.xy);let d=textureLoad(resolve_depth,q,0);let encoded=textureLoad(resolve_normals,q,0);
    let uv=pixel.xy/resolve.control.xy;
    let w=resolve.inverse*vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,max(d,1e-8),1.0);
    let world=w.xyz/w.w;let n=receiver_normal(world,encoded.xyz*2.0-1.0);
    shadow_footprint=shadow_receiver_footprint(world);
    if d<=0.0 || encoded.a<0.5 {return vec4(1.0,0.0,0.0,1.0);}
    var visibility=sun_visibility(world,n);
    if resolve.control.z>0.5 {visibility=min(visibility,textureLoad(contact_map,q,0).r);}
    return vec4(visibility,1.0,0.0,1.0);
}
