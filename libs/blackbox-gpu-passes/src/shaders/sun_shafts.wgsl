// Directional-light visibility in the shared scattering grid.
@group(2) @binding(3) var cascades: texture_depth_2d_array;
@group(2) @binding(4) var detail: texture_depth_2d_array;

@group(2) @binding(5) var cloud: texture_2d<f32>;

fn cloud_visibility(p:vec3<f32>,sun:vec3<f32>)->f32 {
    if shadows.cloud.w<0.5 || sun.z<=0.05 { return 1.0; }
    let uv=(p.xy-p.z*sun.xy/sun.z-shadows.cloud.xy)/8000.0+0.5;
    let edge=min(uv,1.0-uv);
    if min(edge.x,edge.y)<=0.0 { return 1.0; }
    let transmission=textureLoad(cloud,vec2<i32>(uv*vec2<f32>(textureDimensions(cloud))),0).r;
    return mix(1.0,transmission,smoothstep(0.0,0.04,min(edge.x,edge.y))*smoothstep(0.05,0.15,sun.z));
}

fn sun_visibility(p:vec3<f32>)->f32 {
    if shadows.options.x < 0.5 { return 1.0; }
    let depth=dot(p-shadows.camera.xyz,shadows.forward.xyz);
    for(var i=0u;i<u32(shadows.camera.w);i++) {
        if depth>shadows.splits[i] { continue; }
        let h=shadows.matrices[i]*vec4(p,1.0);
        let q=h.xyz/h.w;
        let uv=q.xy*vec2(0.5,-0.5)+0.5;
        if any(uv<vec2(0.0)) || any(uv>=vec2(1.0)) || q.z<0.0 || q.z>1.0 { continue; }
        if shadows.softness.w>0.5 && i<2u { return textureSampleCompareLevel(detail,shadow_sampler,uv,i32(i),q.z-0.00003); }
        let layer=i32(i)-select(0,2,shadows.softness.w>0.5);
        return textureSampleCompareLevel(cascades,shadow_sampler,uv,layer,q.z-0.00003);
    }
    return 1.0;
}

