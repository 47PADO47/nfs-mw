// AO-only history, reprojected with the previous depth of the actual moving surface.
@group(0) @binding(2) var current_ao: texture_2d<f32>;
@group(0) @binding(6) var history_ao: texture_2d<f32>;
@group(0) @binding(7) var object_motion: texture_2d<f32>;
@group(0) @binding(8) var accumulated: texture_storage_2d<rgba16float, write>;
struct Reprojection { previous_from_current: mat4x4<f32>, control: vec4<f32> };
@group(0) @binding(9) var<uniform> reprojection: Reprojection;

// Half-resolution samples sit at ao_pixel(p), not the normalized texel centers of
// an arbitrary resized texture. Odd final rows/columns have a one-pixel spacing.
fn history_coordinate(uv: vec2<f32>) -> vec2<f32> {
    let pixel=uv*params.sizes.xy-0.5;
    let step=f32(params.sampling.x);
    let offset=f32(params.sampling.x/2u);
    let size=params.sizes.zw;
    let prior=(size-2.0)*step+offset;
    let last=min((size-1.0)*step+offset,params.sizes.xy-1.0);
    let ordinary=(pixel-offset)/step;
    let tail=size-2.0+(pixel-prior)/max(last-prior,vec2<f32>(1.0));
    return select(ordinary,select(ordinary,tail,pixel>prior),last<(size-1.0)*step+offset);
}

@compute @workgroup_size(8,8)
fn accumulate(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=vec2<i32>(params.sizes.zw);
    let p=vec2<i32>(id.xy);
    if any(p>=size) { return; }
    let value=textureLoad(current_ao,p,0);
    var visibility=value.x;
    let pixel=ao_pixel(p);
    let motion=textureLoad(object_motion,pixel,0);
    if reprojection.control.x>0.5 && value.y>0.0 && motion.w>0.5 && motion.z>0.0 {
        let uv=(vec2<f32>(pixel)+0.5)/params.sizes.xy + motion.xy*vec2<f32>(0.5,-0.5) + reprojection.control.zw;
        if all(uv>=vec2<f32>(0.0)) && all(uv<vec2<f32>(1.0)) {
            let location=history_coordinate(uv);
            let base=vec2<i32>(floor(location));
            let fraction=fract(location);
            let normal=normalize((reprojection.previous_from_current*vec4<f32>(decode_normal(value.zw),0.0)).xyz);
            let expected=view_at(uv*params.sizes.xy-0.5,motion.z);
            var sum=0.0; var total=0.0;
            for(var y=0;y<=1;y++) { for(var x=0;x<=1;x++) {
                let q=base+vec2<i32>(x,y);
                if any(q<vec2<i32>(0)) || any(q>=size) { continue; }
                let previous=textureLoad(history_ao,q,0);
                if previous.y<=0.0 || dot(normal,decode_normal(previous.zw))<=0.9 { continue; }
                // Compare the previous surface plane, not raw depth at a neighboring ray.
                // A sloping road has different depths across the four bilinear taps.
                let old_point=view_at(vec2<f32>(ao_pixel(q)),previous.y);
                if abs(dot(old_point-expected,normal))>=max(0.04,motion.z*0.002) { continue; }
                let weight=select(1.0-fraction,fraction,vec2<i32>(x,y)!=vec2<i32>(0));
                let w=weight.x*weight.y;
                sum+=previous.x*w; total+=w;
            }}
            if total>0.001 {
                var low=value.x; var high=value.x;
                for(var y=-1;y<=1;y++) { for(var x=-1;x<=1;x++) {
                    let neighbor=textureLoad(current_ao,clamp(p+vec2<i32>(x,y),vec2<i32>(0),size-1),0);
                    if abs(neighbor.y-value.y)<max(0.04,value.y*0.01) && dot(decode_normal(neighbor.zw),decode_normal(value.zw))>0.9 {
                        low=min(low,neighbor.x); high=max(high,neighbor.x);
                    }
                }}
                // A tiny noisy current neighborhood must not erase the accumulated estimate
                // every frame. Keep a bounded margin; fully open surfaces still clear at once.
                if low<0.995 {
                    visibility=mix(value.x,clamp(sum/total,max(0.0,low-0.035),min(1.0,high+0.035)),reprojection.control.y);
                }
            }
        }
    }
    textureStore(accumulated,p,vec4<f32>(visibility,value.yzw));
}
