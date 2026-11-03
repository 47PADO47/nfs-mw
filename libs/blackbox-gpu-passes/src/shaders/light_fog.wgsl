// Integrate the shared scattering volume once per pixel, with extinction.
struct Globals { view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>, fog_color: vec4<f32>, fog_range: vec4<f32> };
struct Shelter { bounds: vec4<f32>, info: vec4<f32>, points: array<vec4<f32>,8> };
struct Params { inverse: mat4x4<f32>, options: vec4<f32>, grid: vec4<u32>, previous_view_proj: mat4x4<f32>, previous_camera: vec4<f32>, temporal: vec4<f32>, shadow_indices: array<vec4<u32>,32>, positions: array<vec4<f32>,128>, directions: array<vec4<f32>,128>, colors: array<vec4<f32>,128>, uppers: array<vec4<f32>,128>, shelters: array<Shelter,24> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var depth: texture_depth_2d;
@group(1) @binding(1) var<uniform> params: Params;
@group(1) @binding(2) var<storage,read> cells: array<vec4<f32>>;
@group(2) @binding(0) var fog: texture_2d<f32>;

@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
    let p=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(p[i],0.0,1.0);
}
fn ray_end(uv:vec2<f32>,z:f32)->vec3<f32> {
    let h=params.inverse*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),max(z,0.000001),1.0);
    return h.xyz/h.w;
}
fn scene_distance(uv:vec2<f32>)->f32 {
    let size=vec2<f32>(textureDimensions(depth));
    let z=textureLoad(depth,vec2<i32>(clamp(uv,vec2(0.0),vec2(0.999999))*size),0);
    return min(length(ray_end(uv,z)-globals.camera_pos.xyz),params.options.w);
}
fn sample_cell(uv:vec2<f32>,slice:u32)->vec4<f32> {
    let p=uv*vec2<f32>(params.grid.xy)-0.5;
    let base=vec2<i32>(floor(p));
    let f=fract(p);
    var value=vec4(0.0);
    for(var y=0;y<2;y++) { for(var x=0;x<2;x++) {
        let tile=vec2<u32>(clamp(base+vec2(x,y),vec2(0),vec2<i32>(params.grid.xy)-vec2(1)));
        let weight=select(1.0-f.x,f.x,x==1)*select(1.0-f.y,f.y,y==1);
        value+=cells[(slice*params.grid.y+tile.y)*params.grid.x+tile.x]*weight;
    }}
    return value;
}
// The half-resolution image covers ceil(size/2) pixels; both passes map its texels to uv the same way.
fn half_size()->vec2<u32> { return (textureDimensions(depth)+vec2(1u))/2u; }
@fragment fn fs_fog(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> {
    let uv=clamp(pixel.xy/vec2<f32>(half_size()),vec2(0.0),vec2(0.999999));
    let end=scene_distance(uv);
    let location=sqrt(end/params.options.w)*f32(params.grid.z);
    let slice=min(u32(location),params.grid.z-1u);
    let a=f32(slice)/f32(params.grid.z);
    let b=f32(slice+1u)/f32(params.grid.z);
    let fraction=clamp((end-params.options.w*a*a)/max(params.options.w*(b*b-a*a),0.00001),0.0,1.0);
    var before=vec4(0.0,0.0,0.0,1.0);
    if slice>0u { before=sample_cell(uv,slice-1u); }
    // Alpha carries the texel's scene distance to the composite; transmission is not needed there.
    return vec4(mix(before,sample_cell(uv,slice),fraction).rgb,end);
}
fn composite(pixel:vec4<f32>)->vec4<f32> {
    let size=vec2<f32>(textureDimensions(depth));
    let uv=clamp(pixel.xy/size,vec2(0.0),vec2(0.999999));
    let distance=scene_distance(uv);
    let low_size=vec2<i32>(textureDimensions(fog));
    let sample_pos=uv*vec2<f32>(low_size)-0.5;
    let base=vec2<i32>(floor(sample_pos));
    let frac=fract(sample_pos);
    var sum=vec3(0.0);
    var weight=0.0;
    for(var y=0;y<2;y++) { for(var x=0;x<2;x++) {
        let coord=clamp(base+vec2(x,y),vec2(0),low_size-vec2(1));
        let f=textureLoad(fog,coord,0);
        let bilinear=select(1.0-frac.x,frac.x,x==1)*select(1.0-frac.y,frac.y,y==1);
        let w=bilinear*exp(-abs(f.a-distance)*0.35);
        sum+=f.rgb*w; weight+=w;
    }}
    // Alpha never reaches the target (the blend keeps the destination's).
    if weight<0.00001 { return vec4(0.0,0.0,0.0,1.0); }
    return vec4(sum/weight,1.0);
}
@fragment fn fs_composite(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> { return composite(pixel); }
@fragment fn fs_composite_srgb(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> {
    let c=composite(pixel);
    return vec4(select(pow((c.rgb+0.055)/1.055,vec3(2.4)),c.rgb/12.92,c.rgb<=vec3(0.04045)),c.a);
}
