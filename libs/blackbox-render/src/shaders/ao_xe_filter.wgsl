// XeGTAO symmetric edge-aware spatial denoising, adapted to WGSL / FP16 intermediates.
// Copyright (C) 2016-2021, Intel Corporation
// SPDX-License-Identifier: MIT
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
// Source: GameTechDev/XeGTAO, a5b1686c7ea37788eeb3576b5be47f7c03db532c,
// Source/Rendering/Shaders/XeGTAO.hlsli: UnpackEdges, Denoise (spatial passes).
@group(0) @binding(2) var source_ao: texture_2d<f32>;
@group(0) @binding(3) var filtered_ao: texture_storage_2d<rgba16float,write>;
override FINAL_PASS: bool = true;

fn xe_load(pixel: vec2<i32>) -> vec4<f32> {
    return textureLoad(source_ao,clamp(pixel,vec2<i32>(0),vec2<i32>(params.sizes.zw)-1),0);
}
fn unpack_edges(data: vec4<f32>) -> vec4<f32> {
    let packed=u32(data.z+0.5);
    return vec4<f32>(f32((packed>>6u)&3u),f32((packed>>4u)&3u),f32((packed>>2u)&3u),f32(packed&3u))/3.0;
}

@compute @workgroup_size(8,8)
fn denoise_xe(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(params.sizes.zw)) { return; }
    let p=vec2<i32>(id.xy);
    let data=xe_load(p);
    if data.y == 0.0 {
        textureStore(filtered_ao,id.xy,vec4<f32>(1.0,0.0,0.0,0.0));
        return;
    }
    let left=xe_load(p+vec2<i32>(-1,0));
    let right=xe_load(p+vec2<i32>(1,0));
    let top=xe_load(p+vec2<i32>(0,-1));
    let bottom=xe_load(p+vec2<i32>(0,1));
    let l=unpack_edges(left);
    let r=unpack_edges(right);
    let t=unpack_edges(top);
    let b=unpack_edges(bottom);
    var edges=unpack_edges(data)*vec4<f32>(l.y,r.x,t.w,b.z);
    let leak=clamp(1.5-dot(edges,vec4<f32>(1.0)),0.0,1.0)*(0.5/1.5);
    edges=clamp(edges+leak,vec4<f32>(0.0),vec4<f32>(1.0));
    let diagonal=0.425*vec4<f32>(edges.x*l.z+edges.z*t.x,edges.z*t.y+edges.y*r.z,
                                edges.w*b.x+edges.x*l.w,edges.y*r.w+edges.w*b.y);
    let beta=select(0.24,1.2,FINAL_PASS);
    var sum=data.x*beta+dot(vec4<f32>(left.x,right.x,top.x,bottom.x),edges);
    sum+=dot(vec4<f32>(xe_load(p+vec2<i32>(-1,-1)).x,xe_load(p+vec2<i32>(1,-1)).x,
                       xe_load(p+vec2<i32>(-1,1)).x,xe_load(p+vec2<i32>(1,1)).x),diagonal);
    let weight=beta+dot(edges,vec4<f32>(1.0))+dot(diagonal,vec4<f32>(1.0));
    if !FINAL_PASS {
        textureStore(filtered_ao,id.xy,vec4<f32>(sum/weight,data.yzw));
        return;
    }
    let pixel=ao_pixel(p);
    let center=view_at(vec2<f32>(pixel),data.y);
    textureStore(filtered_ao,id.xy,vec4<f32>(sum/weight,data.y,encode_normal(normal_at(pixel,center))));
}
