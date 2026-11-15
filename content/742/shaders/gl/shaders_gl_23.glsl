


struct VS_OUT {
    vec4 MulColour;
    vec4 AddColour;
};

struct PS_OUT {
    vec4 Colour;
};

VS_OUT TMP2;
PS_OUT ret_0;
vec3 TMP24;
float x0029;
float TMP34;
float b0039;
vec3 b0041;
float TMP46;
float a0047;
vec4 a0053;
float x0055;
float TMP56;
float b0061;
uniform sampler2D DiffuseSampler;
uniform samplerCube EnvironmentSampler;
uniform vec3 DistanceFogColour;
uniform vec4 HeightFogPlane;
uniform vec3 HeightFogColour;
uniform vec4 SpecularExponent;

 void main()
{

    PS_OUT Out;
    vec4 DiffuseColour;
    vec3 SpecColour1;
    vec3 TMP1;

    TMP2.MulColour = gl_Color;
    TMP2.AddColour = gl_SecondaryColor;
    DiffuseColour = texture2D(DiffuseSampler, gl_TexCoord[0].xy);
    a0047 = dot(gl_TexCoord[3].xyz, gl_TexCoord[3].xyz);
    TMP46 = 1.00000000E+00/inversesqrt(a0047);
    if ((ShaderMode == 1 || ShaderMode == 5) && TMP46 > 0.00000000E+00) {         TMP24 = gl_TexCoord[1].xyz - (2.00000000E+00*gl_TexCoord[3].xyz)*dot(gl_TexCoord[3].xyz, gl_TexCoord[1].xyz);
        x0029 = dot(TMP24, gl_TexCoord[2].xyz);
        b0039 = min(1.00000000E+00, x0029);
        TMP34 = max(0.00000000E+00, b0039);
        SpecColour1 = vec3(pow(TMP34, SpecularExponent.x), pow(TMP34, SpecularExponent.x), pow(TMP34, SpecularExponent.x));
        b0041 = DiffuseColour.xyz*gl_Color.xyz;
        TMP1 = ((SpecColour1*gl_TexCoord[2].w)*(vec3( 1.00000000E+00, 1.00000000E+00, 1.00000000E+00) + SpecularExponent.y*(b0041 - vec3( 1.00000000E+00, 1.00000000E+00, 1.00000000E+00))))*DiffuseColour.w;
        TMP2.AddColour = gl_SecondaryColor + vec4(TMP1.x, TMP1.y, TMP1.z, 0.00000000E+00);
    }     if (ShaderMode == 2) {         TMP2.AddColour.xyz = TMP2.AddColour.xyz + textureCube(EnvironmentSampler, gl_TexCoord[1].xyz).xyz*DiffuseColour.w*TMP2.AddColour.w;
        TMP2.MulColour.xyz = gl_Color.xyz*vec3(1.00000000E+00 - DiffuseColour.w, 1.00000000E+00 - DiffuseColour.w, 1.00000000E+00 - DiffuseColour.w);
    }     if (IgnoreAlpha) {         DiffuseColour.w = 1.00000000E+00;
    }     if (ShaderMode == 4 || ShaderMode == 5) {         a0053 = vec4(0.00000000E+00, gl_TexCoord[0].z, 0.00000000E+00, 0.00000000E+00);
        x0055 = dot(a0053, HeightFogPlane);
        b0061 = min(1.00000000E+00, x0055);
        TMP56 = max(0.00000000E+00, b0061);
        TMP2.MulColour.xyz = TMP2.MulColour.xyz*(1.00000000E+00 - TMP56);
        TMP2.AddColour.xyz = TMP2.AddColour.xyz*(1.00000000E+00 - TMP56) + HeightFogColour.xyz*TMP56;
        TMP2.MulColour.xyz = TMP2.MulColour.xyz*(1.00000000E+00 - gl_TexCoord[0].w);
        TMP2.AddColour.xyz = TMP2.AddColour.xyz*(1.00000000E+00 - gl_TexCoord[0].w) + DistanceFogColour.xyz*gl_TexCoord[0].w;
    }     Out.Colour = DiffuseColour*TMP2.MulColour + vec4(TMP2.AddColour.x, TMP2.AddColour.y, TMP2.AddColour.z, 0.00000000E+00);
    if (ShaderMode == 2 || ShaderMode == 1 || ShaderMode == 5) {         Out.Colour.w = gl_Color.w;
    }     ret_0.Colour = Out.Colour;
    gl_FragColor = Out.Colour;
    return;
} 