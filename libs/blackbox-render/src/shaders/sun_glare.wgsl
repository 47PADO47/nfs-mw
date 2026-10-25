// Main-view lens artwork. Test visibility at the solar source, not across the glare quad.
struct Params { sun:vec4<f32>, screen:vec4<f32>, colour:vec4<f32>, layer:vec4<f32>, cloud_uv:vec4<f32> };
@group(0) @binding(0) var<uniform> params:Params;
@group(0) @binding(1) var scene_depth:texture_depth_2d;
@group(0) @binding(2) var cloud_map:texture_2d<f32>;
@group(1) @binding(0) var artwork:texture_2d<f32>;
@group(1) @binding(1) var art_sampler:sampler;
struct Out { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) @interpolate(flat) visibility:f32 };
fn cloud_visibility()->f32 {
    if params.screen.w<0.5 {return params.layer.w;}
    let size=vec2<i32>(textureDimensions(cloud_map));let p=params.cloud_uv.xy*vec2<f32>(size)-0.5;let f=fract(p);let base=vec2<i32>(floor(p));
    var sum=0.0;
    for(var y=0;y<=1;y++){for(var x=0;x<=1;x++) {
        let q=clamp(base+vec2(x,y),vec2(0),size-vec2(1));let w=select(1.0-f,f,vec2(x,y)!=vec2(0));
        sum+=textureLoad(cloud_map,q,0).a*w.x*w.y;
    }}
    return clamp(sum,0.0,1.0);
}
@vertex fn vs(@builtin(vertex_index)i:u32)->Out {
    let corners=array<vec2<f32>,6>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(1.0,1.0),vec2(-1.0,-1.0),vec2(1.0,1.0),vec2(-1.0,1.0));
    let uv=params.sun.xy*vec2(0.5,-0.5)+0.5;let size=vec2<i32>(params.screen.xy);
    var visibility=0.0;
    for(var y=-1;y<=1;y++){for(var x=-1;x<=1;x++) {
        let p=vec2<i32>(uv*params.screen.xy+vec2<f32>(f32(x),f32(y))*params.screen.z);
        if any(p<vec2(0))||any(p>=size){continue;}
        if textureLoad(scene_depth,p,0)<=0.0 {visibility+=1.0/9.0;}
    }}
    visibility*=cloud_visibility()*params.sun.w;
    var out:Out;out.uv=corners[i]*vec2(0.5,-0.5)+0.5;out.visibility=visibility;
    let c=corners[i];let angle=params.layer.z;let rotated=vec2(c.x*cos(angle)-c.y*sin(angle),c.x*sin(angle)+c.y*cos(angle));
    let offset=rotated*params.layer.x*2.0*vec2(params.screen.y/params.screen.x,1.0);
    out.position=vec4(params.sun.xy+offset,0.0,1.0);
    if visibility<0.001 {out.position=vec4(2.0,2.0,0.0,1.0);}
    return out;
}
@fragment fn fs(in:Out)->@location(0)vec4<f32> {
    let art=textureSample(artwork,art_sampler,in.uv);
    return vec4(art.rgb*art.a*params.colour.rgb*params.colour.a*params.layer.y*in.visibility,0.0);
}
