enable wgpu_ray_query;
// Central or finite-disc sun visibility. World-anchored samples have no alternating frame seed.
struct RtVertex { position: vec4<f32>, normal: vec4<f32>, colour: vec4<f32>, uv: vec4<f32> };
@group(0) @binding(1) var rt_scene: acceleration_structure;
@group(0) @binding(2) var<storage,read> rt_vertices: array<RtVertex>;
@group(0) @binding(3) var<storage,read> rt_triangles: array<vec4<u32>>;
@group(0) @binding(4) var rt_textures: texture_2d_array<f32>;
@group(0) @binding(5) var rt_sampler: sampler;
struct RtControl { scene: vec4<u32>, tuning: vec4<f32> };
@group(0) @binding(6) var<uniform> rt_control: RtControl;
@group(0) @binding(9) var rt_detail_textures: texture_2d_array<f32>;
struct Params { inverse: mat4x4<f32>, view_proj: mat4x4<f32>, camera: vec4<f32>, sun: vec4<f32>, screen: vec4<f32>, tuning: vec4<f32>, soft: vec4<f32> };
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var depth: texture_depth_2d;
@group(1) @binding(2) var normals: texture_2d<f32>;
fn unproject(uv: vec2<f32>, d: f32) -> vec3<f32> {
    let p = params.inverse * vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,d,1.0);
    return p.xyz/p.w;
}
fn alpha(layer:u32,uv:vec2<f32>)->f32 {
    if (layer&32768u)!=0u { return textureSampleLevel(rt_detail_textures,rt_sampler,uv,i32(layer&32767u),0.0).a; }
    return textureSampleLevel(rt_textures,rt_sampler,uv,i32(layer),0.0).a;
}
@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
    let p=vec2(f32((i<<1u)&2u),f32(i&2u)); return vec4(p*2.0-1.0,0.0,1.0);
}
fn sun_visibility_ray(origin:vec3<f32>,ray:vec3<f32>,bias:f32)->vec2<f32> {
    var query:ray_query;
    rayQueryInitialize(&query,rt_scene,RayDesc(0u,2u,bias*0.5,params.sun.w,origin,ray));
    while rayQueryProceed(&query) {
        let candidate=rayQueryGetCandidateIntersection(&query);
        let t=rt_triangles[candidate.instance_custom_data+candidate.primitive_index];
        let a=rt_vertices[t.x]; let b=rt_vertices[t.y]; let c=rt_vertices[t.z];
        let mode=u32(a.normal.w+0.1);
        if mode!=0u && !(mode==4u && params.tuning.y>0.5) {
            let bary=vec3(1.0-candidate.barycentrics.x-candidate.barycentrics.y,candidate.barycentrics);
            let uv=a.uv.xy*bary.x+b.uv.xy*bary.y+c.uv.xy*bary.z;
            let vertex_alpha=dot(vec3(a.colour.a,b.colour.a,c.colour.a),bary);
            var opacity=alpha(t.w&65535u,uv)*vertex_alpha;
            if params.tuning.y>0.5 {
                if mode==2u { opacity=alpha(t.w&65535u,uv); }
                if mode==3u { opacity*=2.0; }
            }
            let world_cutout = mode==5u || mode==3u;
            if select(opacity<0.5,opacity<=0.375,world_cutout) { continue; }
        }
        // Shadow maps also draw casters without back-face culling.
        rayQueryConfirmIntersection(&query);
    }
    let hit=rayQueryGetCommittedIntersection(&query);
    return vec2(select(1.0,0.0,hit.kind!=0u),select(0.0,hit.t,hit.kind!=0u));
}
fn sun_random(p:vec2<u32>,i:u32,phase:u32)->f32 {
    var h=p.x*1664525u+p.y*1013904223u+i*2246822519u+phase*3266489917u;
    h=(h^(h>>16u))*2246822519u;h=h^(h>>13u);
    return f32(h&16777215u)/16777216.0;
}
@fragment fn fs_main(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> {
    let coordinate=min(vec2<i32>(pixel.xy/params.tuning.zw*params.screen.xy),vec2<i32>(params.screen.xy)-vec2(1));
    let d=textureLoad(depth,coordinate,0);
    let encoded=textureLoad(normals,coordinate,0);
    let world=unproject((vec2<f32>(coordinate)+0.5)/params.screen.xy,max(d,1e-8));
    let plane=cross(dpdx(world),dpdy(world));
    // Missing/incomplete scenes must retain cascades, never imply unoccluded sunlight.
    if d<=0.0 || encoded.a<0.5 || rt_control.scene.y==0u { return vec4(1.0,0.0,0.0,1.0); }
    let weight=1.0-smoothstep(params.camera.w*0.8,params.camera.w,distance(world,params.camera.xyz));
    if weight<=0.0 { return vec4(1.0,0.0,0.0,1.0); }
    let n=normalize(encoded.xyz*2.0-1.0);
    // Face the geometric plane toward the light for thin, two-sided foliage.
    let geometric=select(n,plane/max(length(plane),1e-6),dot(plane,plane)>1e-12);
    let normal=select(-geometric,geometric,dot(geometric,params.sun.xyz)>=0.0);
    let bias=params.tuning.x;
    let origin=world+normal*bias;
    if params.soft.x<=0.0 {
        let sample=sun_visibility_ray(origin,params.sun.xyz,bias);
        return vec4(sample.x,weight,sample.y,1.0);
    }
    // Rotate +Z to the sun. The basis stays continuous throughout the daylight hemisphere,
    // unlike choosing a different cross-product axis at an angular threshold.
    let denominator=max(1.0+params.sun.z,0.000001);
    let tangent=normalize(vec3(1.0-params.sun.x*params.sun.x/denominator,
        -params.sun.x*params.sun.y/denominator,-params.sun.x));
    let bitangent=cross(params.sun.xyz,tangent);
    let count=u32(params.soft.y);
    let lattice=bitcast<vec3<u32>>(vec3<i32>(floor(world*32.0)));
    let seed=vec2(lattice.x^(lattice.z*2246822519u),lattice.y^(lattice.z*3266489917u));
    var visible=0.0; var separation=0.0; var hits=0.0;
    for(var i=0u;i<count;i++) {
        let radius=sqrt(sun_random(seed,i*2u+1u,u32(params.soft.w)))*params.soft.x;
        let angle=6.28318531*(f32(i)+sun_random(seed,i*2u,u32(params.soft.w)))/f32(count);
        let ray=normalize(params.sun.xyz+radius*(cos(angle)*tangent+sin(angle)*bitangent));
        let sample=sun_visibility_ray(origin,ray,bias);
        visible+=sample.x;
        if sample.x<0.5 { separation+=sample.y; hits+=1.0; }
    }
    return vec4(visible/f32(count),weight,separation/max(hits,1.0),1.0);
}
