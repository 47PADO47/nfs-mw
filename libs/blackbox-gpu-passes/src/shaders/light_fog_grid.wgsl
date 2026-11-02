// Camera-aligned fog cells share light evaluation between nearby pixels.
struct Globals { view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>, fog_color: vec4<f32>, fog_range: vec4<f32> };
struct Shelter { bounds: vec4<f32>, info: vec4<f32>, points: array<vec4<f32>,8> };
struct Params { inverse: mat4x4<f32>, options: vec4<f32>, grid: vec4<u32>, previous_view_proj: mat4x4<f32>, previous_camera: vec4<f32>, temporal: vec4<f32>, shadow_indices: array<vec4<u32>,32>, positions: array<vec4<f32>,128>, directions: array<vec4<f32>,128>, colors: array<vec4<f32>,128>, uppers: array<vec4<f32>,128>, shelters: array<Shelter,24> };
struct ShadowData {
    matrices: array<mat4x4<f32>,4>, splits: vec4<f32>, options: vec4<f32>,
    camera: vec4<f32>, forward: vec4<f32>, sun: vec4<f32>, texels: vec4<f32>,
    depth_spans: vec4<f32>, softness: vec4<f32>, contact: vec4<f32>, ambient: vec4<f32>,
    environment: vec4<f32>, world: vec4<f32>, debug: vec4<f32>, projected: vec4<f32>,
    resolutions: vec4<f32>, cloud: vec4<f32>, world_light: array<vec4<f32>,9>,
    local_matrices: array<mat4x4<f32>,16>, local_positions: array<vec4<f32>,128>,
    local_directions: array<vec4<f32>,128>, local_colors: array<vec4<f32>,128>,
    local_uppers: array<vec4<f32>,128>, local_weights: array<vec4<f32>,4>,
    local_options: vec4<f32>, local_tuning: vec4<f32>, local_grid: array<vec4<u32>,256>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var<storage,read_write> cells: array<vec4<f32>>;
@group(1) @binding(2) var<storage,read_write> masks: array<vec4<u32>>;
@group(1) @binding(3) var<storage,read> history: array<vec4<f32>>;
@group(1) @binding(4) var<storage,read_write> integrated: array<vec4<f32>>;
@group(2) @binding(0) var<uniform> shadows: ShadowData;
@group(2) @binding(1) var shadow_map: texture_depth_2d_array;
@group(2) @binding(2) var shadow_sampler: sampler_comparison;

fn shadow_visibility(i:u32,p:vec3<f32>)->f32 {
    let slot=params.shadow_indices[i/4u][i%4u];
    if slot>=16u || slot>=u32(shadows.local_options.y) { return 1.0; }
    let weight=shadows.local_weights[slot/4u][slot%4u];
    if weight<=0.0 { return 1.0; }
    let clip=shadows.local_matrices[slot]*vec4(p,1.0);
    if clip.w<=0.0 { return 1.0; }
    let position=clip.xyz/clip.w;
    let uv=position.xy*vec2(0.5,-0.5)+0.5;
    if position.z<0.0 || position.z>1.0 || any(uv<vec2(0.0)) || any(uv>vec2(1.0)) { return 1.0; }
    return mix(1.0,textureSampleCompareLevel(shadow_map,shadow_sampler,uv,i32(slot),position.z-0.00003),weight);
}

fn reproject(p:vec3<f32>,current:vec4<f32>)->vec4<f32> {
    if params.temporal.x<=0.0 { return current; }
    let clip=params.previous_view_proj*vec4(p,1.0);
    if clip.w<=0.0 { return current; }
    let uv=clip.xy/clip.w*vec2(0.5,-0.5)+vec2(0.5);
    let distance=length(p-params.previous_camera.xyz);
    if any(uv<vec2(0.0)) || any(uv>=vec2(1.0)) || distance>=params.temporal.y { return current; }
    let location=vec3(uv*vec2<f32>(params.grid.xy),sqrt(distance/params.temporal.y)*f32(params.grid.z))-0.5;
    let base=vec3<i32>(floor(location));
    let fraction=fract(location);
    var old=vec4(0.0);
    for(var z=0;z<2;z++) { for(var y=0;y<2;y++) { for(var x=0;x<2;x++) {
        let cell=vec3<u32>(clamp(base+vec3(x,y,z),vec3(0),vec3<i32>(params.grid.xyz)-vec3(1)));
        let weight=select(1.0-fraction.x,fraction.x,x==1)*select(1.0-fraction.y,fraction.y,y==1)*select(1.0-fraction.z,fraction.z,z==1);
        let previous=history[(cell.z*params.grid.y+cell.y)*params.grid.x+cell.x];
        // Unlit tiles store zero; their medium matched the current one, and their light was zero.
        old+=select(previous,vec4(0.0,0.0,0.0,current.a),previous.a<=0.0)*weight;
    }}}
    let difference=length(old.rgb-current.rgb)/max(max(length(current.rgb),length(old.rgb)),0.0001);
    let density_difference=abs(old.a-current.a)/max(max(current.a,old.a),0.0001);
    let confidence=1.0-smoothstep(0.10,0.35,max(difference,density_difference));
    return mix(current,old,params.temporal.x*confidence);
}

fn shelter_point(s:u32,i:u32)->vec2<f32> {
    let pair=params.shelters[s].points[i/2u];
    return select(pair.xy,pair.zw,(i%2u)==1u);
}
fn weather_shelter(p:vec3<f32>)->f32 {
    var shelter=0.0;
    for(var s=0u;s<params.grid.w;s++) {
        let region=params.shelters[s];
        if any(p.xy<region.bounds.xy) || any(p.xy>region.bounds.zw) || p.z>region.info.x+1.0 { continue; }
        let count=u32(region.info.y);
        if count<3u { continue; }
        var inside=false;
        var edge_distance=100000.0;
        var previous=shelter_point(s,count-1u);
        for(var i=0u;i<count;i++) {
            let current=shelter_point(s,i);
            let edge=previous-current;
            let t=clamp(dot(p.xy-current,edge)/max(dot(edge,edge),0.00001),0.0,1.0);
            edge_distance=min(edge_distance,length(p.xy-current-edge*t));
            if (current.y>p.y)!=(previous.y>p.y) {
                let x=current.x+(p.y-current.y)/(previous.y-current.y)*(previous.x-current.x);
                if p.x<x { inside=!inside; }
            }
            previous=current;
        }
        if inside {
            let below=1.0-smoothstep(region.info.x-1.0,region.info.x+1.0,p.z);
            shelter=max(shelter,smoothstep(0.0,3.0,edge_distance)*below);
        }
    }
    return shelter;
}

// Conservative projected sphere bounds, including lights crossing the near plane.
fn affects_tile(i:u32, tile:vec2<u32>)->bool {
    let p=params.positions[i];
    if distance(globals.camera_pos.xyz,p.xyz)-p.w>params.options.w { return false; }
    let clip=globals.view_proj*vec4(p.xyz,1.0);
    let row_w=vec3(globals.view_proj[0].w,globals.view_proj[1].w,globals.view_proj[2].w);
    let w_radius=length(row_w)*p.w;
    if clip.w+w_radius<=0.0 { return false; }
    if clip.w<=w_radius+0.001 { return true; }
    let ndc=clip.xy/clip.w;
    let row_x=vec3(globals.view_proj[0].x,globals.view_proj[1].x,globals.view_proj[2].x);
    let row_y=vec3(globals.view_proj[0].y,globals.view_proj[1].y,globals.view_proj[2].y);
    let radius=p.w*(vec2(length(row_x),length(row_y))+abs(ndc)*length(row_w))/(clip.w-w_radius);
    let center=ndc*vec2(0.5,-0.5)+vec2(0.5);
    let low=vec2<f32>(tile)/vec2<f32>(params.grid.xy);
    let high=vec2<f32>(tile+vec2(1u))/vec2<f32>(params.grid.xy);
    let r=radius*0.5;
    return all(center+r>=low) && all(center-r<=high);
}
@compute @workgroup_size(8,8,1)
fn tile_lights(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=params.grid.xy) { return; }
    var mask=vec4<u32>(0u);
    for(var i=0u;i<u32(params.options.z);i++) {
        if affects_tile(i,id.xy) { mask[i/32u] |= 1u << (i%32u); }
    }
    masks[id.y*params.grid.x+id.x]=mask;
}
@compute @workgroup_size(8,8,1)
fn build_volume(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id>=params.grid.xyz) { return; }
    let index=(id.z*params.grid.y+id.y)*params.grid.x+id.x;
    let mask=masks[id.y*params.grid.x+id.x];
    // No light reaches this column: its scattered light is zero and its transmission never reaches the image.
    if all(mask==vec4(0u)) { cells[index]=vec4(0.0); return; }
    let uv=(vec2<f32>(id.xy)+0.5)/vec2<f32>(params.grid.xy);
    let h=params.inverse*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.000001,1.0);
    let direction=normalize(h.xyz/h.w-globals.camera_pos.xyz);
    let slice=(f32(id.z)+0.5)/f32(params.grid.z);
    let t=params.options.w*slice*slice;
    let p=globals.camera_pos.xyz+direction*t;
    let density=exp(clamp(-(p.z-globals.camera_pos.z)*0.035,-2.0,2.0));
    let weather=params.options.y*(1.0-0.85*weather_shelter(p));
    let amount=params.options.x*mix(0.12,0.65,weather)*density;
    var light=vec3(0.0);
    for(var word=0u;word<4u;word++) {
        var bits=mask[word];
        while bits!=0u {
            let bit=firstTrailingBit(bits);
            let i=word*32u+bit;
            bits &= bits-1u;
            let delta=p-params.positions[i].xyz;
            let d2=dot(delta,delta);
            let range=params.positions[i].w;
            if d2>=range*range || d2<0.0001 { continue; }
            let ray_to_sample=delta*inverseSqrt(d2);
            // Directions and upper planes arrive normalized.
            var cone=smoothstep(params.directions[i].w,params.colors[i].w,dot(ray_to_sample,params.directions[i].xyz));
            if params.uppers[i].w>0.5 { cone*=1.0-smoothstep(0.10,0.30,dot(ray_to_sample,params.uppers[i].xyz)); }
            let edge=max(1.0-d2/(range*range),0.0);
            let fade=1.0-smoothstep(params.options.w*0.8,params.options.w,t);
            light+=params.colors[i].rgb*cone*edge*edge/(1.0+0.04*d2)*fade*shadow_visibility(i,p);
        }
    }
    let current=vec4(light*amount*0.012,amount*0.004);
    // An unlit cell takes no history colour (any lit history fails the confidence test), so skip its loads.
    if all(light==vec3(0.0)) { cells[index]=current; return; }
    cells[index]=reproject(p,current);
}

// Integrate each screen column once, rather than repeating the same work for every pixel.
@compute @workgroup_size(8,8,1)
fn integrate_volume(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=params.grid.xy) { return; }
    // An unlit column adds nothing; the composite ignores transmission.
    if all(masks[id.y*params.grid.x+id.x]==vec4(0u)) {
        for(var z=0u;z<params.grid.z;z++) { integrated[(z*params.grid.y+id.y)*params.grid.x+id.x]=vec4(0.0,0.0,0.0,1.0); }
        return;
    }
    var sum=vec3(0.0);
    var transmission=1.0;
    for(var z=0u;z<params.grid.z;z++) {
        let a=f32(z)/f32(params.grid.z);
        let b=f32(z+1u)/f32(params.grid.z);
        let step=params.options.w*(b*b-a*a);
        let index=(z*params.grid.y+id.y)*params.grid.x+id.x;
        let medium=cells[index];
        let extinction=exp(-medium.a*step);
        let integral=(1.0-extinction)/max(medium.a,0.000001);
        sum+=transmission*medium.rgb*integral;
        transmission*=extinction;
        integrated[index]=vec4(sum,transmission);
    }
}
