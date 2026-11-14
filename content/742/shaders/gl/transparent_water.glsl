




vec4 ret_0;
vec3 TMP26;
float x0031;
vec3 TMP32;
float x0041;
float TMP44;
float b0049;
float x0051;
float TMP52;
float b0057;
float x0059;
float x0069;
float TMP70;
float b0075;
vec4 b0079;
uniform vec3 DistanceFogColour;
uniform vec4 SunDirectionAndExponent;
uniform vec4 SunColourAndWaveExponent;
uniform vec4 WaveIntensityAndBreakWaterDepthAndOffset;
uniform sampler3D NormalSampler;
uniform samplerCube EnvMapSampler;

 void main()
{

    vec4 Normal;
    vec3 EnvColour;
    float SpecularIntensity;
    float WaveFactor;
    float NdotE;
    float Fresnel;
    float DiffuseIntensity;
    float mainAlpha;
    vec4 SurfaceColour;
    vec4 MainColour1;
    vec4 WaveColour1;
    vec4 waterColour1;
    vec3 TMP2;
    vec3 TMP4;

    Normal = texture3D(NormalSampler, gl_TexCoord[0].xyz);
    Normal.xyz = 2.00000000E+00*Normal.xyz - 1.00000000E+00;
    x0031 = dot(gl_TexCoord[1].xyz, gl_TexCoord[1].xyz);
    TMP26 = inversesqrt(x0031)*gl_TexCoord[1].xyz;
    TMP32 = TMP26 - (2.00000000E+00*Normal.xyz)*dot(Normal.xyz, TMP26);
    EnvColour = textureCube(EnvMapSampler, TMP32).xyz;
    x0041 = dot(SunDirectionAndExponent.xyz, TMP32);
    b0049 = min(1.00000000E+00, x0041);
    TMP44 = max(0.00000000E+00, b0049);
    SpecularIntensity = pow(TMP44, SunDirectionAndExponent.w);
    x0051 = gl_TexCoord[0].w/WaveIntensityAndBreakWaterDepthAndOffset.z - WaveIntensityAndBreakWaterDepthAndOffset.w*Normal.w;
    b0057 = min(1.00000000E+00, x0051);
    TMP52 = max(0.00000000E+00, b0057);
    x0059 = 1.00010002E+00 - TMP52;
    WaveFactor = pow(x0059, SunColourAndWaveExponent.w) - 5.00000000E-01;
    WaveFactor = -4.00000000E+00*WaveFactor*WaveFactor + 1.00000000E+00;
    NdotE = dot(TMP32, Normal.xyz);
    Fresnel = 1.00000000E+00 - abs(NdotE);
    DiffuseIntensity = dot(SunDirectionAndExponent.xyz, Normal.xyz)*2.00000000E+00*(1.00000000E+00 - Fresnel);
    x0069 = gl_TexCoord[0].w/4.00000000E+01;
    b0075 = min(1.00000000E+00, x0069);
    TMP70 = max(0.00000000E+00, b0075);
    mainAlpha = Fresnel*TMP52*TMP70*TMP70*(3.00000000E+00 - 2.00000000E+00*TMP70);
    if (waves) {         MainColour1 = vec4(EnvColour.x, EnvColour.y, EnvColour.z, mainAlpha);
        WaveColour1 = vec4(WaveIntensityAndBreakWaterDepthAndOffset.x*Normal.w + WaveIntensityAndBreakWaterDepthAndOffset.y, WaveIntensityAndBreakWaterDepthAndOffset.x*Normal.w + WaveIntensityAndBreakWaterDepthAndOffset.y, WaveIntensityAndBreakWaterDepthAndOffset.x*Normal.w + WaveIntensityAndBreakWaterDepthAndOffset.y, WaveIntensityAndBreakWaterDepthAndOffset.x*Normal.w + WaveIntensityAndBreakWaterDepthAndOffset.y);
        TMP4 = (SpecularIntensity*SunColourAndWaveExponent.xyz)*TMP52;
        SurfaceColour = MainColour1 + WaveFactor*(WaveColour1 - MainColour1) + vec4(TMP4.x, TMP4.y, TMP4.z, 0.00000000E+00);
    } else {
        waterColour1 = vec4(DistanceFogColour.x, DistanceFogColour.y, DistanceFogColour.z, 1.00000000E+00);
        TMP2 = EnvColour + (DiffuseIntensity + SpecularIntensity)*SunColourAndWaveExponent.xyz;
        b0079 = waterColour1*vec4(TMP2.x, TMP2.y, TMP2.z, 1.00000000E+00);
        SurfaceColour = waterColour1 + Fresnel*(b0079 - waterColour1);
    }     ret_0 = SurfaceColour*gl_Color + gl_SecondaryColor;
    gl_FragColor = ret_0;
    return;
} 