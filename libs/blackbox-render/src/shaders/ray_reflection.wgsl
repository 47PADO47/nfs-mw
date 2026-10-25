@group(0) @binding(1) var rt_scene: acceleration_structure;
struct RtVertex { position: vec4<f32>, normal: vec4<f32>, colour: vec4<f32>, uv: vec4<f32> };
@group(0) @binding(2) var<storage,read> rt_vertices: array<RtVertex>;
@group(0) @binding(3) var<storage,read> rt_triangles: array<vec4<u32>>;
@group(0) @binding(4) var rt_textures: texture_2d_array<f32>;
@group(0) @binding(5) var rt_sampler: sampler;
struct RtControl { scene: vec4<u32>, tuning: vec4<f32> };
@group(0) @binding(6) var<uniform> rt_control: RtControl;
@group(0) @binding(7) var<storage,read> rt_winding: array<u32>;
@group(0) @binding(9) var rt_detail_textures: texture_2d_array<f32>;

fn rt_tex(triangle:vec4<u32>,bary:vec3<f32>,lod:f32)->vec4<f32> {
    let a=rt_vertices[triangle.x]; let b=rt_vertices[triangle.y]; let c=rt_vertices[triangle.z];
    let uv=a.uv.xy*bary.x+b.uv.xy*bary.y+c.uv.xy*bary.z;
    return rt_sample(triangle.w&65535u,uv,lod);
}
fn rt_sample(layer:u32,uv:vec2<f32>,lod:f32)->vec4<f32> {
    if (layer&32768u)!=0u { return textureSampleLevel(rt_detail_textures,rt_sampler,uv,i32(layer&32767u),clamp(lod+log2(f32(textureDimensions(rt_detail_textures).x)/128.0),0.0,f32(textureNumLevels(rt_detail_textures)-1u))); }
    return textureSampleLevel(rt_textures,rt_sampler,uv,i32(layer),clamp(lod,0.0,7.0));
}
struct RtHitMaterial {
    diffuse_min: vec4<f32>, diffuse_range: vec4<f32>, specular_min: vec4<f32>,
    specular_range: vec4<f32>, params: vec4<f32>, surface: vec4<f32>, live: vec4<f32>
};
@group(0) @binding(8) var<storage,read> rt_materials: array<RtHitMaterial>;
fn rt_linear(c:vec3<f32>)->vec3<f32> {
    let v=max(c,vec3(0.0)); return select(pow((v+0.055)/1.055,vec3(2.4)),v/12.92,v<=vec3(0.04045));
}
fn rt_encoded(c:vec3<f32>)->vec3<f32> {
    let v=max(c,vec3(0.0)); return select(1.055*pow(v,vec3(1.0/2.4))-0.055,12.92*v,v<=vec3(0.0031308));
}
fn rt_glossy_hit(m:RtHitMaterial,tex:vec3<f32>,colour:vec4<f32>,n:vec3<f32>,v:vec3<f32>,position:vec3<f32>)->vec3<f32> {
    let facing=clamp(dot(n,v),0.0,1.0);
    let diffuse=max(m.diffuse_min+m.diffuse_range*facing,vec4(0.0));
    let occlusion=clamp(2.0*colour.r,0.0,1.0);
    let enhanced=m.surface.w>0.5 && shadow_data.environment.w>0.5;
    let kind=select(0u,u32(m.surface.w+0.1),enhanced);
    let roughness=select(0.15,m.surface.x,enhanced);
    var env=vec3(0.0); var reflected=vec3(0.0); var fresnel=vec3(0.0);
    if kind!=10u {
        env=rt_hit_environment(position,reflect(-v,n),roughness);
        var f0=select(vec3(0.04),vec3(0.025),kind==9u);
        if kind==5u || kind==11u {
            let albedo=select(tex*diffuse.rgb,tex,kind==11u);
            f0=clamp(mix(vec3(0.65),rt_linear(albedo),select(0.35,0.60,kind==11u)),vec3(0.06),vec3(0.95));
        }
        fresnel=f0+(max(vec3(1.0-roughness),f0)-f0)*pow(1.0-facing,5.0);
        let intensity=select(1.0,m.surface.z,enhanced);
        reflected=rt_encoded(rt_linear(env)*fresnel*intensity*occlusion*diffuse.a);
        // Conductors have no diffuse lobe, so do not waste shadow samples on one.
        if kind==5u || kind==11u { return reflected; }
    }
    let ambient=select(vec3(0.35),globals.surface_ambient.rgb*globals.fog_color.w,globals.surface_ambient.w>0.5);
    let sun_colour=select(vec3(0.65),globals.surface_diffuse.rgb,globals.surface_ambient.w>0.5);
    let sun=max(dot(n,-globals.light_dir.xyz),0.0)*capture_sun_visibility(position,n);
    let base=tex*diffuse.rgb*(ambient+sun_colour*sun)*occlusion;
    if kind==10u { return base; }
    if kind==6u { return base*0.25+reflected; }
    if enhanced { return base*(1.0-max(fresnel.r,max(fresnel.g,fresnel.b)))+reflected; }
    let gain=max(m.params.y+m.params.z*pow(facing,m.params.w),0.0)*0.5*diffuse.a;
    return base+env*gain*occlusion;
}
struct RtHit { radiance: vec3<f32>, position: vec3<f32>, distance: f32, found: bool };
fn trace_environment(world:vec3<f32>,normal:vec3<f32>,ray:vec3<f32>,bias:f32,range:f32)->RtHit {
    let origin=world+normal*bias;
    var query:ray_query;
    rayQueryInitialize(&query,rt_scene,RayDesc(0u,1u,0.025,range,origin,ray));
    while rayQueryProceed(&query) {
        let candidate=rayQueryGetCandidateIntersection(&query);
        let triangle=rt_triangles[candidate.instance_custom_data+candidate.primitive_index];
        if (triangle.w&(1u<<25u))!=0u {
            // Match raster sidedness in object space. A mirrored transform cancels
            // raster winding reversal; only the explicit baked-winding flag remains.
            let a=rt_vertices[triangle.x].position.xyz;
            let b=rt_vertices[triangle.y].position.xyz;
            let c=rt_vertices[triangle.z].position.xyz;
            let local_ray=candidate.world_to_object*vec4(ray,0.0);
            let front=dot(cross(b-a,c-a),local_ray)<0.0;
            if front==(rt_winding[candidate.instance_index]!=0u) { continue; }
        }
        let family=((triangle.w>>16u)&255u)|((triangle.w>>26u)<<8u);
        if family>=3u && rt_materials[family-3u].live.x<0.5 { continue; }
        let bary=vec3(1.0-candidate.barycentrics.x-candidate.barycentrics.y,candidate.barycentrics);
        if (triangle.w&(1u<<24u))!=0u && rt_tex(triangle,bary,0.0).a<0.5 { continue; }
        rayQueryConfirmIntersection(&query);
    }
    let hit=rayQueryGetCommittedIntersection(&query);
    if hit.kind==0u { return RtHit(vec3(0.0),world,0.0,false); }
    let triangle=rt_triangles[hit.instance_custom_data+hit.primitive_index];
    let bary=vec3(1.0-hit.barycentrics.x-hit.barycentrics.y,hit.barycentrics);
    let a=rt_vertices[triangle.x]; let b=rt_vertices[triangle.y]; let c=rt_vertices[triangle.z];
    let colour=a.colour*bary.x+b.colour*bary.y+c.colour*bary.z;
    var local_n=a.normal.xyz*bary.x+b.normal.xyz*bary.y+c.normal.xyz*bary.z;
    let lod=log2(max(hit.t*0.004,0.0001));
    if a.uv.z>0.0 {
        let uv=a.uv.xy*bary.x+b.uv.xy*bary.y+c.uv.xy*bary.z;
        let bump=rt_sample(u32(a.uv.z)-1u,uv,lod).rgb*2.0-vec3(1.0);
        let du=b.uv.xy-a.uv.xy; let dv=c.uv.xy-a.uv.xy;
        let determinant=du.x*dv.y-du.y*dv.x;
        if abs(determinant)>1e-8 && dot(local_n,local_n)>1e-8 {
            let n0=normalize(local_n);
            let raw=((b.position.xyz-a.position.xyz)*dv.y-(c.position.xyz-a.position.xyz)*du.y)/determinant;
            let orthogonal=raw-n0*dot(raw,n0);
            if dot(orthogonal,orthogonal)>1e-8 {
                let tangent=normalize(orthogonal);
                let bitangent=cross(n0,tangent)*sign(determinant);
                local_n=tangent*bump.x+bitangent*bump.y+n0*bump.z;
            }
        }
    }
    let n=normalize(transpose(mat3x3(hit.world_to_object[0],hit.world_to_object[1],hit.world_to_object[2]))*local_n);
    let position=origin+ray*hit.t;
    let tex=rt_tex(triangle,bary,lod);
    let kind=((triangle.w>>16u)&255u)|((triangle.w>>26u)<<8u);
    var radiance=2.0*tex.rgb*colour.rgb;
    if kind!=0u && kind<3u {
        let ambient=select(vec3(0.35),globals.surface_ambient.rgb*globals.fog_color.w,globals.surface_ambient.w>0.5);
        let diffuse=select(vec3(0.65),globals.surface_diffuse.rgb,globals.surface_ambient.w>0.5);
        let visible=capture_sun_visibility(position,n);
        radiance=2.0*tex.rgb*(ambient*colour.rgb+diffuse*max(dot(n,-globals.light_dir.xyz),0.0)*visible);
    }
    if kind>=3u { radiance=rt_glossy_hit(rt_materials[kind-3u],tex.rgb,colour,n,-ray,position); }
    return RtHit(radiance,position,hit.t,true);
}
