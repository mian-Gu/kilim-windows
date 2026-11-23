// Rendering/Renderer.swift shaders, with a cropped native Windows output surface.
cbuffer Uniforms : register(b0) {
    float2 viewSize;
    float2 surfaceSize;
    float2 origin;
    float2 texel;
    float alpha;
    float pointScale;
    float2 padding;
    float4 lightDirection;
    float4 tint;
    float4 shadow;
    float foldContrast;
};
Texture2D image : register(t0);
SamplerState imageSampler : register(s0);
struct Input { float3 position : POSITION; float2 uv : TEXCOORD0; float3 normal : NORMAL; float ao : TEXCOORD1; };
struct RugOut { float4 position : SV_POSITION; float2 uv : TEXCOORD0; float3 normal : NORMAL; float ao : TEXCOORD1; };
float4 project(float2 screenPosition, float depth) {
    float2 p = screenPosition - origin;
    return float4(p.x / surfaceSize.x * 2 - 1, 1 - p.y / surfaceSize.y * 2, depth, 1);
}
RugOut rugVS(Input input) {
    RugOut o;
    float3 p = input.position;
    o.position = project(p.xy + (p.xy - viewSize * 0.5) * p.z * 0.0009 / pointScale, 0.5 - p.z / padding.y);
    o.uv = input.uv; o.normal = input.normal; o.ao = input.ao;
    return o;
}
float4 rugFS(RugOut input) : SV_TARGET {
    float4 t = image.Sample(imageSampler, input.uv);
    if (t.a < 0.4) discard;
    // Which side of the cloth is on show comes from the surface normal, not from the winding of the
    // projected triangle: winding is decided per triangle after projection, so a single triangle on
    // a crease can flip it while its neighbours do not.
    float3 n = normalize(input.normal + float3(0, 0, 1e-6));
    bool front = n.z >= 0.;
    float3 normal = front ? n : -n;
    float3 facing = t.rgb;
    // The rug's underside is the same weave seen through the pile, so it keeps the desaturated
    // version of the print. A printed sheet is not: its back is bare cloth, with only a hint of the
    // print showing through. That also keeps any sliver of underside revealed at a tight crease from
    // reading as a hole punched in the panel. padding.x is the full-screen flag.
    float3 weave = lerp(facing, dot(facing, float3(0.3,0.59,0.11)).xxx, 0.38) * 0.80 + float3(0.035,0.025,0.01);
    float3 bare = float3(0.87, 0.84, 0.78) * (0.93 + 0.07 * dot(facing, float3(0.3,0.59,0.11)));
    float3 turned = lerp(weave, bare, padding.x);
    // Cross into the back colour gradually instead of switching at the exact silhouette. A real fold
    // is edge-on and lands fully on the back colour, but a pinch a few pixels wide that briefly turns
    // the surface over keeps the front colour, so it no longer reads as a hole in the cloth.
    float3 color = lerp(turned, facing, smoothstep(-0.45, 0.25, n.z));
    float3 light = normalize(lightDirection.xyz);
    float diffuse = max(dot(normal, light), 0);
    float ambient = lerp(0.42, 0.70, padding.x);
    float lit = ambient + (1. - ambient) * clamp(diffuse / light.z, 0, 1.4);
    lit = lerp(0.62, lit, lightDirection.w);
    float3 halfVector = normalize(light + float3(0,0,1));
    float specular = pow(max(dot(normal, halfVector), 0), 28) * 0.10 * lightDirection.w;
    float occlusion = lerp(input.ao, 1., padding.x * 0.75);
    float3 highlight = float3(1.0,0.92,0.8);
    float3 shaded = color * lit * occlusion * tint.rgb + specular * highlight;
    if (padding.x > 0. && foldContrast < 1.) {
        // Use the lighting of a flat surface as the reference, while keeping the
        // actual side of the cloth, its pattern and its opacity.
        float flatLight = lerp(0.62, 1., lightDirection.w);
        float flatSpecular = pow(max(halfVector.z, 0.), 28) * 0.10 * lightDirection.w;
        float3 flat = color * flatLight * tint.rgb + flatSpecular * highlight;
        shaded = lerp(flat, shaded, foldContrast);
    }
    return float4(shaded * alpha, alpha);
}
struct ShadowOut { float4 position : SV_POSITION; float2 uv : TEXCOORD0; float intensity : TEXCOORD1; };
ShadowOut shadowVS(Input input) {
    ShadowOut o;
    o.position = project(input.position.xy + shadow.xy * input.position.z + shadow.zw, 0);
    o.uv = input.uv; o.intensity = exp(-input.position.z / (55*pointScale));
    return o;
}
float4 shadowFS(ShadowOut input) : SV_TARGET {
    if (image.Sample(imageSampler, input.uv).a < 0.35) discard;
    return input.intensity.xxxx;
}
struct FullOut { float4 position : SV_POSITION; float2 uv : TEXCOORD0; };
FullOut fullVS(uint id : SV_VertexID) {
    float2 p = float2((id << 1) & 2, id & 2);
    FullOut o; o.position = float4(p * 2 - 1,0,1); o.uv = float2(p.x,1-p.y); return o;
}
float4 compositeFS(FullOut input) : SV_TARGET {
    float a = image.Sample(imageSampler, input.uv).r * 0.20;
    float weight = 0.20;
    [unroll] for (int i = 0; i < 12; i++) {
        float angle = i * 0.5235988;
        float radius = i % 2 == 0 ? 1.6 : 3.2;
        float2 offset = float2(cos(angle),sin(angle)) * radius * texel;
        float w = i % 2 == 0 ? 0.10 : 0.05;
        a += image.Sample(imageSampler,input.uv + offset).r * w; weight += w;
    }
    a = min((a / weight) * 0.62 * tint.w,0.62) * alpha;
    return float4(0,0,0,a);
}
