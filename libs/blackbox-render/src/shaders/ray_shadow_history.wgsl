// History always stores the CURRENT UNBLENDED spatial result, never the resolved result.
struct Params { previous_inverse:mat4x4<f32>, screen:vec4<f32>, control:vec4<f32>, jitter:vec4<f32> };
@group(0) @binding(0) var<uniform> params:Params;
@group(0) @binding(1) var source:texture_2d<f32>;
@group(0) @binding(2) var history:texture_2d<f32>;
@group(0) @binding(3) var depth:texture_depth_2d;
@group(0) @binding(4) var normals:texture_2d<f32>;
@group(0) @binding(5) var motion:texture_2d<f32>;
fn nonzero_sign(v:vec2<f32>)->vec2<f32>{return select(vec2(-1.0),vec2(1.0),v>=vec2(0.0));}
fn encode(n:vec3<f32>)->vec2<f32>{let v=n/(abs(n.x)+abs(n.y)+abs(n.z));return select((1.0-abs(v.yx))*nonzero_sign(v.xy),v.xy,v.z>=0.0);}
fn decode(e:vec2<f32>)->vec3<f32>{var v=vec3(e,1.0-abs(e.x)-abs(e.y));if v.z<0.0 {v=vec3((1.0-abs(v.yx))*nonzero_sign(v.xy),v.z);}return normalize(v);}
fn previous_point(uv:vec2<f32>,z:f32)->vec3<f32>{let w=params.previous_inverse*vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,params.control.z/z,1.0);return w.xyz/w.w;}
@vertex fn vs(@builtin(vertex_index)i:u32)->@builtin(position)vec4<f32>{let p=vec2(f32((i<<1u)&2u),f32(i&2u));return vec4(p*2.0-1.0,0.0,1.0);}
struct Out{@location(0) mask:vec4<f32>,@location(1) next:vec4<f32>};
@fragment fn fs(@builtin(position)pixel:vec4<f32>)->Out {
    let p=vec2<i32>(pixel.xy);let size=vec2<i32>(params.screen.xy);
    let value=textureLoad(source,p,0);let d=textureLoad(depth,p,0);let encoded=textureLoad(normals,p,0);
    var out:Out;out.mask=vec4(value.rg,0.0,1.0);out.next=vec4(0.0);
    if d<=0.0 || encoded.a<0.5 || value.g<0.98 {return out;}
    let n=normalize(encoded.xyz*2.0-1.0);let z=params.screen.z/d;
    out.next=vec4(value.r,z,encode(n));
    let m=textureLoad(motion,p,0);
    if params.control.x<0.5 || m.w<0.5 || m.z<=0.0 {return out;}
    let uv=pixel.xy/params.screen.xy+m.xy*vec2(0.5,-0.5)+params.jitter.xy;
    if any(uv<vec2(0.0)) || any(uv>=vec2(1.0)) {return out;}
    let location=uv*params.screen.xy-0.5;let base=vec2<i32>(floor(location));let fraction=fract(location);
    let expected=previous_point(uv,m.z);let tolerance=max(0.02,z*params.screen.w*2.0);
    var sum=0.0;var total=0.0;
    for(var y=0;y<=1;y++){for(var x=0;x<=1;x++) {
        let q=base+vec2(x,y);if any(q<vec2(0)) || any(q>=size){continue;}
        let old=textureLoad(history,q,0);
        if old.g<=0.0 || dot(n,decode(old.ba))<0.97 {continue;}
        let old_world=previous_point((vec2<f32>(q)+0.5)/params.screen.xy,old.g);
        if abs(dot(old_world-expected,n))>tolerance {continue;}
        let w=select(1.0-fraction,fraction,vec2(x,y)!=vec2(0));sum+=old.r*w.x*w.y;total+=w.x*w.y;
    }}
    if total<=0.001 {return out;}
    let old=sum/total;
    // Large lighting/blocker changes reject history immediately, even on an unchanged receiver.
    if abs(old-value.r)>0.2 {return out;}
    var low=value.r;var high=value.r;
    for(var y=-1;y<=1;y++){for(var x=-1;x<=1;x++) {
        let neighbor=textureLoad(source,clamp(p+vec2(x,y),vec2(0),size-vec2(1)),0);
        low=min(low,neighbor.r);high=max(high,neighbor.r);
    }}
    if low>=0.999 || high<=0.001 {return out;}
    out.mask.r=mix(value.r,clamp(old,max(0.0,low-0.02),min(1.0,high+0.02)),params.control.y);
    return out;
}
