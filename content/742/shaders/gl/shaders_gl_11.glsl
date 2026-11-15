


struct VS_OUT {
    vec4 Position;
    vec4 TexCoordAndWaterDepth;
    vec4 MulColour;
    vec4 AddColour;
    vec3 ViewVector;
};

VS_OUT ret_0;
vec4 r0018;
vec2 r0020;
vec3 TMP21;
vec3 v0022;
float x0026;
float x0030;
float TMP31;
float b0036;
float x0042;
float TMP43;
float b0048;
uniform vec4 WVPMatrix[4];
uniform vec2 TexCoordMatrix[4];
uniform vec4 EyePosAndTime;
uniform vec4 DistanceFogPlane;
uniform vec3 DistanceFogColour;
uniform vec4 HeightFogPlane;
uniform vec3 HeightFogColour;

 void main()
{

    VS_OUT Out;

    Out.TexCoordAndWaterDepth.w = gl_MultiTexCoord1.x;
    r0018 = gl_Vertex.x*WVPMatrix[0];
    r0018 = r0018 + gl_Vertex.y*WVPMatrix[1];
    r0018 = r0018 + gl_Vertex.z*WVPMatrix[2];
    r0018 = r0018 + gl_Vertex.w*WVPMatrix[3];
    r0020 = gl_MultiTexCoord0.x*TexCoordMatrix[0];
    r0020 = r0020 + gl_MultiTexCoord0.y*TexCoordMatrix[1];
    r0020 = r0020 + gl_MultiTexCoord0.z*TexCoordMatrix[2];
    r0020 = r0020 + gl_MultiTexCoord0.w*TexCoordMatrix[3];
    Out.TexCoordAndWaterDepth.xy = r0020.xy;
    Out.TexCoordAndWaterDepth.z = EyePosAndTime.w;
    v0022 = EyePosAndTime.xyz - gl_Vertex.xyz;
    x0026 = dot(v0022, v0022);
    TMP21 = inversesqrt(x0026)*v0022;
    x0030 = dot(gl_Vertex, HeightFogPlane);
    b0036 = min(1.00000000E+00, x0030);
    TMP31 = max(0.00000000E+00, b0036);
    Out.MulColour.xyz = vec3(1.00000000E+00 - TMP31, 1.00000000E+00 - TMP31, 1.00000000E+00 - TMP31);
    Out.AddColour.xyz = TMP31*HeightFogColour.xyz;
    x0042 = dot(gl_Vertex, DistanceFogPlane);
    b0048 = min(1.00000000E+00, x0042);
    TMP43 = max(0.00000000E+00, b0048);
    Out.MulColour.xyz = Out.MulColour.xyz*(1.00000000E+00 - TMP43);
    Out.AddColour.xyz = Out.AddColour.xyz*(1.00000000E+00 - TMP43) + DistanceFogColour.xyz*TMP43;
    Out.MulColour.w = 1.00000000E+00;
    Out.AddColour.w = 0.00000000E+00;
    ret_0.Position = r0018;
    ret_0.TexCoordAndWaterDepth = Out.TexCoordAndWaterDepth;
    ret_0.MulColour = Out.MulColour;
    ret_0.AddColour = Out.AddColour;
    ret_0.ViewVector = TMP21;
    gl_FrontColor = Out.MulColour;
    gl_TexCoord[1].xyz = TMP21;
    gl_FrontSecondaryColor = Out.AddColour;
    gl_Position = r0018;
    gl_TexCoord[0] = Out.TexCoordAndWaterDepth;
    return;
} 