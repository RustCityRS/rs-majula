


struct VS_OUT {
    vec4 Position1;
    vec4 TexCoordAndWaterDepth;
    vec4 MulColour;
    vec4 AddColour;
    vec3 ReflectedViewVector;
    vec4 SpecularColour;
    vec3 Normal1;
};



VS_OUT ret_0;
vec4 r0024;
vec2 r0026;
float x0032;
float TMP33;
float b0038;
float TMP39;
float b0044;
vec3 TMP45;
vec3 v0046;
float x0050;
vec3 TMP51;
float x0056;
vec3 TMP57;
vec3 v0058;
float x0062;
vec3 TMP63;
vec3 r0066;
vec3 TMP69;
float x0074;
float x0078;
float TMP79;
float b0084;
float TMP85;
float x0092;
float TMP93;
float b0098;
float x0102;
float TMP103;
float b0108;
float x0114;
float TMP115;
float b0120;
uniform vec4 WVPMatrix[4];
uniform vec2 TexCoordMatrix[4];
uniform vec3 EyePos;
uniform vec4 DistanceFogPlane;
uniform vec3 DistanceFogColour;
uniform vec4 HeightFogPlane;
uniform vec3 HeightFogColour;
uniform vec3 SunDir;
uniform vec3 SunColour;
uniform vec3 AntiSunColour;
uniform vec3 AmbientColour;
uniform vec3 WorldMatrix[3];
uniform vec4 PointLightsPosAndRadiusSq[4];
uniform vec4 PointLightsDiffuseColour[4];

 void main()
{

    VS_OUT Out;
    vec3 DiffuseColour;
    float NdotL1;
    int i1;
    vec3 LightVec1;
    float LightIntensity1;

    Out.TexCoordAndWaterDepth = vec4( 0.00000000E+00, 0.00000000E+00, 0.00000000E+00, 0.00000000E+00);
    Out.ReflectedViewVector = vec3( 0.00000000E+00, 0.00000000E+00, 0.00000000E+00);
    Out.SpecularColour = vec4( 0.00000000E+00, 0.00000000E+00, 0.00000000E+00, 0.00000000E+00);
    r0024 = gl_Vertex.x*WVPMatrix[0];
    r0024 = r0024 + gl_Vertex.y*WVPMatrix[1];
    r0024 = r0024 + gl_Vertex.z*WVPMatrix[2];
    r0024 = r0024 + gl_Vertex.w*WVPMatrix[3];
    r0026 = gl_MultiTexCoord0.x*TexCoordMatrix[0];
    r0026 = r0026 + gl_MultiTexCoord0.y*TexCoordMatrix[1];
    r0026 = r0026 + gl_MultiTexCoord0.z*TexCoordMatrix[2];
    r0026 = r0026 + gl_MultiTexCoord0.w*TexCoordMatrix[3];
    Out.TexCoordAndWaterDepth.xy = r0026.xy;
    if (ShaderMode == 3) {         DiffuseColour = vec3( 1.00000000E+00, 1.00000000E+00, 1.00000000E+00);
    } else {
        NdotL1 = dot(gl_Normal, SunDir);
        b0038 = min(1.00000000E+00, NdotL1);
        TMP33 = max(0.00000000E+00, b0038);
        x0032 = -NdotL1;
        b0044 = min(1.00000000E+00, x0032);
        TMP39 = max(0.00000000E+00, b0044);
        DiffuseColour = AmbientColour + TMP33*SunColour + TMP39*AntiSunColour;
    }     if (ShaderMode == 1 || ShaderMode == 5) {         Out.SpecularColour.xyz = -SunDir.xyz;
        v0046 = EyePos.xyz - gl_Vertex.xyz;
        x0050 = dot(v0046, v0046);
        TMP45 = inversesqrt(x0050)*v0046;
        Out.ReflectedViewVector = TMP45;
        x0056 = dot(gl_Normal, gl_Normal);
        TMP51 = inversesqrt(x0056)*gl_Normal;
        Out.Normal1 = TMP51;
    } else {
        Out.Normal1 = vec3( 0.00000000E+00, 0.00000000E+00, 0.00000000E+00);
    }     if (ShaderMode == 2) {         v0058 = EyePos.xyz - gl_Vertex.xyz;
        x0062 = dot(v0058, v0058);
        TMP57 = inversesqrt(x0062)*v0058;
        TMP63 = TMP57 - (2.00000000E+00*gl_Normal)*dot(gl_Normal, TMP57);
        r0066 = TMP63.x*WorldMatrix[0];
        r0066 = r0066 + TMP63.y*WorldMatrix[1];
        r0066 = r0066 + TMP63.z*WorldMatrix[2];
        Out.ReflectedViewVector = r0066.xyz;
    }     i1 = 0;
    for (; i1 < PointLightCount; i1 = i1 + 1) {         LightVec1 = PointLightsPosAndRadiusSq[i1].xyz - gl_Vertex.xyz;
        x0074 = dot(LightVec1, LightVec1);
        TMP69 = inversesqrt(x0074)*LightVec1;
        x0078 = dot(gl_Normal, TMP69);
        b0084 = min(1.00000000E+00, x0078);
        TMP79 = max(0.00000000E+00, b0084);
        TMP85 = dot(LightVec1, LightVec1);
        LightIntensity1 = (PointLightsPosAndRadiusSq[i1].w/TMP85)*TMP79;
        DiffuseColour = DiffuseColour + PointLightsDiffuseColour[i1].xyz*LightIntensity1;
    }     DiffuseColour = DiffuseColour*gl_Color.xyz;
    if (ShaderMode == 4 || ShaderMode == 5) {         Out.TexCoordAndWaterDepth.z = gl_MultiTexCoord1.x;
        x0092 = dot(gl_Vertex, DistanceFogPlane);
        b0098 = min(1.00000000E+00, x0092);
        TMP93 = max(0.00000000E+00, b0098);
        Out.TexCoordAndWaterDepth.w = TMP93;
        Out.MulColour.xyz = DiffuseColour;
        Out.AddColour = vec4( 0.00000000E+00, 0.00000000E+00, 0.00000000E+00, 1.00000000E+00);
        if (ShaderMode == 5) {             Out.SpecularColour.w = 1.00000000E+00 - TMP93;
        }     } else {
        x0102 = dot(gl_Vertex, HeightFogPlane);
        b0108 = min(1.00000000E+00, x0102);
        TMP103 = max(0.00000000E+00, b0108);
        Out.MulColour.xyz = DiffuseColour*(1.00000000E+00 - TMP103);
        Out.AddColour.xyz = TMP103*HeightFogColour.xyz;
        Out.AddColour.w = 1.00000000E+00 - TMP103;
        if (ShaderMode == 1) {             Out.SpecularColour.w = 1.00000000E+00 - TMP103;
        }         x0114 = dot(gl_Vertex, DistanceFogPlane);
        b0120 = min(1.00000000E+00, x0114);
        TMP115 = max(0.00000000E+00, b0120);
        Out.MulColour.xyz = Out.MulColour.xyz*(1.00000000E+00 - TMP115);
        Out.AddColour.xyz = Out.AddColour.xyz*(1.00000000E+00 - TMP115) + DistanceFogColour.xyz*TMP115;
        Out.AddColour.w = Out.AddColour.w*(1.00000000E+00 - TMP115);
        if (ShaderMode == 1) {             Out.SpecularColour.w = Out.SpecularColour.w*(1.00000000E+00 - TMP115);
        }     }     Out.MulColour.w = gl_Color.w;
    ret_0.Position1 = r0024;
    ret_0.TexCoordAndWaterDepth = Out.TexCoordAndWaterDepth;
    ret_0.MulColour = Out.MulColour;
    ret_0.AddColour = Out.AddColour;
    ret_0.ReflectedViewVector = Out.ReflectedViewVector;
    ret_0.SpecularColour = Out.SpecularColour;
    ret_0.Normal1 = Out.Normal1;
    gl_FrontColor = Out.MulColour;
    gl_TexCoord[1].xyz = Out.ReflectedViewVector;
    gl_TexCoord[3].xyz = Out.Normal1;
    gl_FrontSecondaryColor = Out.AddColour;
    gl_Position = r0024;
    gl_TexCoord[2] = Out.SpecularColour;
    gl_TexCoord[0] = Out.TexCoordAndWaterDepth;
    return;
} 