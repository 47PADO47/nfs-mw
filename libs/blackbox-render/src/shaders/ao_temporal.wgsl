// Temporal visibility in linear AO space; geometry/material colour and the UI are never accumulated.
@group(0) @binding(2) var current_ao: texture_2d<f32>;
@group(0) @binding(6) var history_ao: texture_2d<f32>;
@group(0) @binding(7) var object_motion: texture_2d<f32>;
@group(0) @binding(8) var accumulated: texture_storage_2d<rgba16float, write>;
struct Reprojection { previous_from_current: mat4x4<f32>, control: vec4<f32> };
@group(0) @binding(9) var<uniform> reprojection: Reprojection;

@compute @workgroup_size(8,8)
fn accumulate(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=vec2<i32>(params.sizes.zw);
    let p=vec2<i32>(id.xy);
    if any(p>=size) { return; }
    let value=textureLoad(current_ao,p,0);
    var visibility=value.x;
    if reprojection.control.x>0.5 && value.y>0.0 {
        let pixel=ao_pixel(p);
        let mv=textureLoad(object_motion,pixel,0).xy;
        let uv=(vec2<f32>(pixel)+0.5)/params.sizes.xy + mv*vec2<f32>(0.5,-0.5) + reprojection.control.zw;
        let old_p=vec2<i32>(floor(uv*vec2<f32>(size)));
        if all(uv>=vec2<f32>(0.0)) && all(uv<vec2<f32>(1.0)) {
            let previous=textureLoad(history_ao,old_p,0);
            let expected=reprojection.previous_from_current*vec4<f32>(view_at(vec2<f32>(pixel),value.y),1.0);
            let n=normalize((reprojection.previous_from_current*vec4<f32>(decode_normal(value.zw),0.0)).xyz);
            let agree=dot(n,decode_normal(previous.zw));
            let error=abs(previous.y+expected.z);
            if previous.y>0.0 && error<max(0.04,abs(expected.z)*0.01) && agree>0.9 {
                var low=value.x; var high=value.x;
                for(var y=-1;y<=1;y++) { for(var x=-1;x<=1;x++) {
                    let neighbor=textureLoad(current_ao,clamp(p+vec2<i32>(x,y),vec2<i32>(0),size-1),0);
                    if abs(neighbor.y-value.y)<max(0.04,value.y*0.01) {
                        low=min(low,neighbor.x); high=max(high,neighbor.x);
                    }
                }}
                let weight=reprojection.control.y*exp(-length(mv)*8.0);
                visibility=mix(value.x,clamp(previous.x,low,high),weight);
            }
        }
    }
    textureStore(accumulated,p,vec4<f32>(visibility,value.yzw));
}
