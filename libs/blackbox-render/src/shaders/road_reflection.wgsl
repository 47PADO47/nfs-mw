// Current opaque scene only. Rays use receiver normals and actual visible geometry, never HUD/history.
@group(1) @binding(7) var<uniform> road_control: vec4<f32>;
@group(1) @binding(8) var road_colour: texture_2d<f32>;
@group(1) @binding(9) var road_depth: texture_depth_2d;
@group(1) @binding(10) var road_environment: texture_cube<f32>;
@group(1) @binding(11) var road_sampler: sampler;
@group(1) @binding(12) var road_filtered_environment: texture_cube<f32>;

fn road_uv(point:vec3<f32>)->vec3<f32> {
    let c = globals.view_proj*vec4(point,1.0);
    return vec3(c.xy/max(c.w,1e-6)*vec2(0.5,-0.5)+0.5,c.w);
}
fn road_delta(p:vec3<f32>,size:vec2<i32>)->f32 {
    let pixel = clamp(vec2<i32>(p.xy*vec2<f32>(size)),vec2<i32>(0),size-1);
    let d = textureLoad(road_depth,pixel,0);
    if d<=0.0 { return -1e6; }
    return p.z-road_control.y/d;
}
// `footprint` is the receiver's world-space pixel size. The caller takes the derivatives in uniform control
// flow, so this march may run behind per-pixel branches (such as skipping unreflective pixels).
fn road_environment_at(world:vec3<f32>,normal:vec3<f32>,ray:vec3<f32>,fallback:vec3<f32>,pool:f32,footprint:f32)->vec3<f32> {
    if road_control.x<0.5 || globals.fog_range.w>0.5 { return fallback; }
    // A fixed centimetre bias can hit the receiver's own depth texel on distant roads.
    let origin = world+normal*max(0.06,footprint*0.25);
    let size = vec2<i32>(textureDimensions(road_depth));
    var previous = 0.15;
    var previous_delta = -1e6;
    // Preserve sampling density as range increases, rather than stretching the same sparse rays.
    let steps = u32(clamp(ceil(road_control.z*0.6),96.0,480.0));
    for (var i=1u;i<=steps;i++) {
        let f = f32(i)/f32(steps);
        let t = 0.15+f*f*road_control.z;
        let p = road_uv(origin+ray*t);
        if p.z<=road_control.y || any(p.xy<=vec2(0.0)) || any(p.xy>=vec2(1.0)) { return fallback; }
        let delta = road_delta(p,size);
        // A front-to-back crossing is necessary. A ray starting behind foreground geometry is a miss.
        if previous_delta<0.0 && delta>=0.0 {
            var low = previous; var high = t;
            for (var j=0u;j<7u;j++) {
                let middle = (low+high)*0.5;
                let hit = road_uv(origin+ray*middle);
                if road_delta(hit,size)>=0.0 { high=middle; continue; }
                low=middle;
            }
            let hit = road_uv(origin+ray*high);
            let error = road_delta(hit,size);
            let thickness = max(0.12,hit.z*0.002);
            if error>=0.0 && error<thickness {
                let edge = min(min(hit.x,1.0-hit.x),min(hit.y,1.0-hit.y));
                let fade_start = road_control.z*(1.0-road_control.w);
                let confidence = smoothstep(0.0,0.09,edge)*(1.0-smoothstep(fade_start,road_control.z,high));
                let pixel = vec2(1.0)/vec2<f32>(textureDimensions(road_colour));
                let radius = mix(2.0,0.35,pool);
                let c = textureSampleLevel(road_colour,road_sampler,hit.xy,0.0).rgb*4.0;
                let filtered = (c+textureSampleLevel(road_colour,road_sampler,hit.xy+pixel*vec2(radius,0.0),0.0).rgb
                    +textureSampleLevel(road_colour,road_sampler,hit.xy-pixel*vec2(radius,0.0),0.0).rgb
                    +textureSampleLevel(road_colour,road_sampler,hit.xy+pixel*vec2(0.0,radius),0.0).rgb
                    +textureSampleLevel(road_colour,road_sampler,hit.xy-pixel*vec2(0.0,radius),0.0).rgb)/8.0;
                return mix(fallback,filtered,confidence);
            }
        }
        previous=t; previous_delta=delta;
    }
    return fallback;
}
