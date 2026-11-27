// Local lights (street lamps, headlights) on a receiver: their shadow maps, light grid and highlights. Appended to
// `shadow_receiver.wgsl` by `shadow_receiver_source`, whose `shadow_data`, sampler and `SHADOW_GROUP` it shares.
// Direct local lighting only. Authored/prelit colour is not used as a light source.
@group(SHADOW_GROUP) @binding(8) var local_shadow_map: texture_depth_2d_array;
fn local_shadow_visibility(uv:vec2<f32>,depth:f32,index:u32)->f32 {
    let radius=shadow_data.local_options.z;
    if radius<=0.0 {
        return textureSampleCompareLevel(local_shadow_map,shadow_sampler,uv,i32(index),depth);
    }
    // A 3x3 tent (1,2,1)/4 of bilinear compares spaced `radius` texels, folded into four bilinear compares. At one texel
    // each pair of its four texels per axis is one tap at its weighted centre (exact); narrower, the taps move towards
    // the centre; wider, two taps per axis at the old filter's variance approximate it (a slightly softer edge).
    let size=vec2<f32>(textureDimensions(local_shadow_map));
    let p=uv*size-0.5; let f=fract(p);
    var lo=p-vec2(radius*0.70710678); var hi=p+vec2(radius*0.70710678);
    var a=vec2(0.5);
    if radius<=1.0 {
        a=0.75-0.5*f;
        lo=p+(-1.0-f+(2.0-f)/(3.0-2.0*f))*radius; hi=p+(1.0-f+f/(1.0+2.0*f))*radius;
    }
    let b=1.0-a;
    lo=(lo+0.5)/size; hi=(hi+0.5)/size;
    let layer=i32(index);
    return a.x*a.y*textureSampleCompareLevel(local_shadow_map,shadow_sampler,lo,layer,depth)
        +b.x*a.y*textureSampleCompareLevel(local_shadow_map,shadow_sampler,vec2(hi.x,lo.y),layer,depth)
        +a.x*b.y*textureSampleCompareLevel(local_shadow_map,shadow_sampler,vec2(lo.x,hi.y),layer,depth)
        +b.x*b.y*textureSampleCompareLevel(local_shadow_map,shadow_sampler,hi,layer,depth);
}
// A 16x16 world-space grid fitted to the selected light spheres (origin tuning.yz, cell
// edge tuning.w) stores conservative light masks. It covers every light, so a receiver
// outside it, in any view, is beyond the reach of all of them.
fn local_light_mask(world:vec3<f32>, bucket:u32)->u32 {
    let cell=vec2<i32>(floor((world.xy-shadow_data.local_tuning.yz)/max(shadow_data.local_tuning.w,0.001)));
    if any(cell<vec2(0)) || any(cell>=vec2(16)) { return 0u; }
    return shadow_data.local_grid[u32(cell.y*16+cell.x)][bucket];
}
fn local_lighting(world:vec3<f32>,normal:vec3<f32>)->vec3<f32> {
    return local_lights(world,normal,vec3(0.0),0.0,normal,0.0,0.0,0.0).diffuse;
}

// Dielectric GGX directional response, with correlated Smith visibility and Schlick Fresnel.
// alpha2 is perceptual roughness to the fourth power, optionally widened by a pixel footprint.
fn world_highlight(n:vec3<f32>,v:vec3<f32>,l:vec3<f32>,alpha2:f32)->f32 {
    let nl=dot(n,l);
    let nv=dot(n,v);
    if nl<=0.0 || nv<=0.0 { return 0.0; }
    let h=(l+v)/max(length(l+v),1e-4);
    let nh=clamp(dot(n,h),0.0,1.0);
    let vh=clamp(dot(v,h),0.0,1.0);
    let denominator=1.0+nh*nh*(alpha2-1.0);
    let distribution=alpha2/(3.14159265*denominator*denominator);
    let gv=nl*sqrt(nv*nv*(1.0-alpha2)+alpha2);
    let gl=nv*sqrt(nl*nl*(1.0-alpha2)+alpha2);
    let edge=1.0-vh;
    let fresnel=0.04+0.96*edge*edge*edge*edge*edge;
    // The world lights and their blend operate in the legacy display domain.
    return sqrt(distribution*0.5/max(gv+gl,1e-5)*fresnel*nl);
}

// The diffuse term and up to two highlights of the lights reaching a receiver, from one walk of its light list: `specular`
// uses `normal` and `power`, `glint` (skipped when `glint_power` is 0) uses `glint_normal`; all share each light's cone,
// attenuation and shadow visibility (looked up along `normal`). `size` > 0 (car paint) gives each emitter that radius
// in metres, so its mirror image grows as it nears, and leaves the glint brighter before it is compressed.
struct LocalLights { diffuse:vec3<f32>, specular:vec3<f32>, glint:vec3<f32> };
fn local_lights(world:vec3<f32>,normal:vec3<f32>,view:vec3<f32>,power:f32,
    glint_normal:vec3<f32>,glint_power:f32,alpha2:f32,size:f32)->LocalLights {
    var result=LocalLights(vec3(0.0),vec3(0.0),vec3(0.0));
    if shadow_data.local_options.x<0.5 { return result; }
    let shiny=power>0.0 && shadow_data.local_tuning.x>0.0;
    let glinting=shiny && glint_power>0.0;
    let n=normal/max(length(normal),0.0001);
    let g=glint_normal/max(length(glint_normal),0.0001);
    let v=view/max(length(view),0.0001);
    for(var bucket=0u;bucket<4u;bucket++) {
      var mask=local_light_mask(world,bucket);
      loop {
        if mask==0u { break; }
        let bit=firstTrailingBit(mask);
        mask &= mask-1u;
        let i=bucket*32u+bit;
        if i>=u32(shadow_data.local_options.x) { continue; }
        let light=shadow_data.local_positions[i];
        let delta=light.xyz-world; let distance2=dot(delta,delta);
        if distance2<=0.000001 || distance2>=light.w*light.w { continue; }
        let d=sqrt(distance2);
        let l=delta/d;
        let lambert=max(dot(n,l),0.0);
        var cone=smoothstep(shadow_data.local_directions[i].w,shadow_data.local_colors[i].w,
            dot(-l,shadow_data.local_directions[i].xyz));
        if shadow_data.local_uppers[i].w>0.5 {
            cone*=1.0-smoothstep(0.10,0.30,dot(-l,shadow_data.local_uppers[i].xyz));
        }
        let half_sum=l+v;
        let h=half_sum/max(length(half_sum),0.0001);
        var highlight=0.0;
        var glint=0.0;
        if shiny {
            highlight=pow(max(dot(n,h),0.0),max(power,1.0))*lambert;
            if alpha2>0.0 { highlight=world_highlight(n,v,l,alpha2); }
        }
        // A glint is the mirrored emitter. A street lamp's bulb stays visible a little outside its beam, from the side;
        // a headlight (it has an upper cutoff) barely shows outside its beam and never from behind.
        // Car paint (`size` > 0) sees a lamp's lantern head from level with it, too.
        let side=select(select(0.5,0.8,size>0.0),0.05,shadow_data.local_uppers[i].w>0.5);
        let emitter=max(cone,side*smoothstep(select(-0.1,-0.7,size>0.0),select(0.4,0.2,size>0.0),dot(-l,shadow_data.local_directions[i].xyz)));
        // A mirror image keeps the emitter's brightness at any distance; only its size changes. The lobe's peak is
        // therefore not scaled by how much light reaches the surface, so a light next to the road does not saturate
        // into a broad white patch and a distant lamp still leaves a visible streak.
        if glinting {
            var lobe=max(glint_power,1.0);
            if size>0.0 {
                // The emitter's angular radius, halved for the half vector, widens the lobe (headlights are smaller).
                let radius=size*select(1.0,0.5,shadow_data.local_uppers[i].w>0.5)/(2.0*d);
                lobe=1.0/(1.0/lobe+radius*radius);
            }
            glint=pow(max(dot(g,h),0.0),lobe)*select(0.35,1.0,size>0.0)*emitter;
        }
        let lit=lambert*cone>0.0;
        let lit_highlight=highlight*cone>=0.0001;
        let lit_glint=glint>=0.0001;
        if !(lit || lit_highlight || lit_glint) { continue; }
        var visibility=1.0;
        if i<u32(shadow_data.local_options.y) && shadow_data.local_weights[i/4u][i%4u]>0.0 {
            let projected=shadow_data.local_matrices[i]*vec4(world+n*0.012,1.0);
            let p=projected.xyz/max(projected.w,1e-6);
            let uv=p.xy*vec2(0.5,-0.5)+0.5;
            let outside=projected.w<=0.0 || p.z<0.0 || p.z>1.0 || any(uv<vec2(0.0)) || any(uv>vec2(1.0));
            // Outside the shadow frustum only an emitter glint remains (the beam's own terms end at its cone).
            if outside && (lit || lit_highlight) { continue; }
            if !outside {
                visibility=mix(1.0,local_shadow_visibility(uv,p.z-0.00003,i),shadow_data.local_weights[i/4u][i%4u]);
            }
        }
        let falloff=pow(max(1.0-d*d/(light.w*light.w),0.0),2.0)/(1.0+0.04*d*d);
        let colour=shadow_data.local_colors[i].rgb*(cone*falloff*visibility);
        if lit { result.diffuse+=colour*lambert; }
        if lit_highlight { result.specular+=colour*highlight; }
        // The emitter term carries its own cone and fades gently with distance (a lamp far down the road still streaks).
        // On car paint the lamp's size already shrinks it with distance; only the light's reach fades it out.
        let reach=max(1.0-d*d/(light.w*light.w),0.0);
        let glint_falloff=select(reach/(1.0+0.02*d*d),smoothstep(0.0,0.3,reach),size>0.0);
        if lit_glint { result.glint+=shadow_data.local_colors[i].rgb*(glint_falloff*visibility*glint); }
      }
    }
    result.diffuse=result.diffuse/(vec3(1.0)+result.diffuse*0.65);
    result.specular=result.specular/(vec3(1.0)+result.specular*0.65)*shadow_data.local_tuning.x;
    result.glint=result.glint/(vec3(1.0)+result.glint*select(0.65,0.2,size>0.0))*shadow_data.local_tuning.x;
    return result;
}
