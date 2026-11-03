// Surface-aware reconstruction of full- or half-resolution sun visibility.
// Output: visibility, coverage, then the local visibility range (min, max) the history clamp uses.
struct Params { inverse:mat4x4<f32>, view_proj:mat4x4<f32>, camera:vec4<f32>, sun:vec4<f32>, screen:vec4<f32>, tuning:vec4<f32>, soft:vec4<f32> };
@group(0) @binding(0) var<uniform> params:Params;
@group(0) @binding(1) var depth:texture_depth_2d;
@group(0) @binding(2) var normals:texture_2d<f32>;
@group(0) @binding(3) var source:texture_2d<f32>;
// Per ray cell: camera-relative receiver position bits and packed octahedral normal (see ray_shadow.wgsl).
@group(0) @binding(4) var surfaces:texture_2d<u32>;
fn point(p:vec2<i32>,d:f32)->vec3<f32> {
    let uv=(vec2<f32>(p)+0.5)/params.screen.xy;
    let w=params.inverse*vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,d,1.0);return w.xyz/w.w;
}
fn cell(p:vec2<i32>)->vec2<i32> {return clamp(vec2<i32>((vec2<f32>(p)+0.5)/params.screen.xy*params.tuning.zw),vec2(0),vec2<i32>(params.tuning.zw)-vec2(1));}
fn nonzero_sign(v:vec2<f32>)->vec2<f32> {return select(vec2(-1.0),vec2(1.0),v>=vec2(0.0));}
fn decode_normal(e:vec2<f32>)->vec3<f32> {
    var v=vec3(e,1.0-abs(e.x)-abs(e.y));
    if v.z<0.0 {v=vec3((1.0-abs(v.yx))*nonzero_sign(v.xy),v.z);}
    return normalize(v);
}
// `relative` is the receiver's camera-relative position; `c` a ray cell, standing for its representative pixel.
fn compatible(c:vec2<i32>,relative:vec3<f32>,n:vec3<f32>,tolerance:f32)->f32 {
    let s=textureLoad(surfaces,c,0);
    if s.w==0x80008000u {return 0.0;}
    let alignment=dot(n,decode_normal(unpack2x16snorm(s.w)));
    if alignment<0.95 {return 0.0;}
    let plane=abs(dot(bitcast<vec3<f32>>(s.xyz)-relative,n))/tolerance;
    if plane>1.0 {return 0.0;}
    let a2=alignment*alignment;let a4=a2*a2;let a8=a4*a4;
    return exp(-plane*plane*2.0)*a8*a8;
}
// Bilateral 2x2 gather. An unmatched foreground surface retains its cascaded shadow.
fn reconstruct(p:vec2<i32>,relative:vec3<f32>,n:vec3<f32>,tolerance:f32)->vec4<f32> {
    if all(params.screen.xy==params.tuning.zw) {return textureLoad(source,p,0);}
    let location=(vec2<f32>(p)+0.5)/params.screen.xy*params.tuning.zw-0.5;
    let base=vec2<i32>(floor(location));let f=fract(location);
    var sum=vec3(0.0);var total=0.0;
    for(var y=0;y<=1;y++){for(var x=0;x<=1;x++) {
        let q=clamp(base+vec2(x,y),vec2(0),vec2<i32>(params.tuning.zw)-vec2(1));
        let w=select(1.0-f,f,vec2(x,y)!=vec2(0));
        let value=textureLoad(source,q,0);
        // An uncovered ray has zero weight, so it cannot contribute to the reconstruction.
        if value.g<0.001 {continue;}
        let weight=w.x*w.y*compatible(q,relative,n,tolerance)*step(0.001,value.g);
        sum+=value.rgb*weight;total+=weight;
    }}
    if total<=0.001 {return vec4(1.0,0.0,0.0,1.0);}
    return vec4(sum/total,1.0);
}
@vertex fn vs(@builtin(vertex_index)i:u32)->@builtin(position) vec4<f32> {let p=vec2(f32((i<<1u)&2u),f32(i&2u));return vec4(p*2.0-1.0,0.0,1.0);}
@fragment fn fs(@builtin(position)pixel:vec4<f32>)->@location(0)vec4<f32> {
    let p=vec2<i32>(pixel.xy);let high=vec2<i32>(params.screen.xy)-vec2(1);
    let d=textureLoad(depth,p,0);let normal=textureLoad(normals,p,0);
    if d<=0.0 || normal.a<0.5 {return vec4(1.0,0.0,1.0,1.0);}
    let world=point(p,d);let n=normalize(normal.xyz*2.0-1.0);
    let relative=world-params.camera.xyz;
    let footprint=max(params.screen.z/d*params.screen.w,0.0001);
    let tolerance=clamp(footprint*0.5,0.006,0.035);
    let center=reconstruct(p,relative,n,tolerance);
    if center.g<=0.0 {return vec4(center.rg,center.r,center.r);}
    // Coverage is evaluated at the full-resolution receiver, not a neighboring ray.
    let coverage=1.0-smoothstep(params.camera.w*0.8,params.camera.w,distance(world,params.camera.xyz));
    var separation=center.b;
    var low=center.r;var top=center.r;
    for(var i=0;i<12;i++) {
        let angle=f32(i)*2.39996323;let radius=select(select(1.0,4.0,i>=4),12.0,i>=8);
        let q=clamp(p+vec2<i32>(round(vec2(cos(angle),sin(angle))*radius)),vec2(0),high);
        let c=cell(q);
        if compatible(c,relative,n,tolerance)<=0.0 {continue;}
        let value=textureLoad(source,c,0);
        separation=max(separation,value.b);
        low=min(low,value.r);top=max(top,value.r);
    }
    // A uniformly lit or umbral neighbourhood needs no disc: its estimate is already exact.
    if top<=0.001 || low>=0.999 {return vec4(center.r,coverage,center.r,center.r);}
    let scale=params.screen.x/params.tuning.z;
    var world_radius=min(separation*params.soft.x*0.55*params.soft.z*sqrt(scale),footprint*32.0);
    if scale>1.0 && center.r>0.001 && center.r<0.999 {world_radius=max(world_radius,footprint*scale);}
    if world_radius/footprint<0.5 {return vec4(center.r,coverage,low,top);}
    let anchor=select(vec3(0.0,1.0,0.0),vec3(1.0,0.0,0.0),abs(n.y)>0.98);
    let tangent=normalize(cross(anchor,n));let bitangent=cross(n,tangent);
    // Continuous world-space rotation avoids screen-locked noise when the camera moves.
    let rotation=6.28318531*fract(dot(world,vec3(0.75487766,0.56984029,0.438579)));
    var sum=center.r;var total=1.0;
    for(var i=0;i<24;i++) {
        let radius=sqrt((f32(i)+0.5)/24.0);let angle=f32(i)*2.39996323+rotation;
        let sample_world=world+(cos(angle)*tangent+sin(angle)*bitangent)*(world_radius*radius);
        let clip=params.view_proj*vec4(sample_world,1.0);
        if clip.w<=0.0 {continue;}
        let uv=clip.xy/clip.w*vec2(0.5,-0.5)+vec2(0.5);let location=uv*params.screen.xy;
        if any(location<vec2(0.0)) || any(location>=params.screen.xy) {continue;}
        let c=cell(vec2<i32>(location));let value=textureLoad(source,c,0);
        if abs(value.g-center.g)>0.08 {continue;}
        let weight=compatible(c,relative,n,tolerance)*exp(-radius*radius*1.5);
        sum+=value.r*weight;total+=weight;
    }
    let resolved=sum/total;
    return vec4(resolved,coverage,min(low,resolved),max(top,resolved));
}
