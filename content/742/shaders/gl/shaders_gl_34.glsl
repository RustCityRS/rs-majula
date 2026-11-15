








vec4 ret_0;
vec4 baseColour0025;
float caustic0025;
vec4 output0025;
float x0029;
float TMP30;
float b0035;
vec3 a0039;
float TMP40;
float b0045;
uniform sampler2D diffuseTexture;
uniform vec4 waterFogColour;
uniform sampler3D causticSampler3D;
uniform vec2 waterParams;

 void main()
{

    float caustic1;

    caustic1 = texture3D(causticSampler3D, gl_TexCoord[2].xyz).w*waterParams.x;
    baseColour0025 = gl_Color*texture2D(diffuseTexture, gl_TexCoord[0].xy);
    x0029 = gl_TexCoord[2].w*2.49999994E-03;
    b0035 = min(1.00000000E+00, x0029);
    TMP30 = max(0.00000000E+00, b0035);
    caustic0025 = caustic1*TMP30;
    output0025.w = baseColour0025.w;
    b0045 = min(1.00000000E+00, gl_TexCoord[1].x);
    TMP40 = max(0.00000000E+00, b0045);
    a0039 = baseColour0025.xyz + caustic0025;
    output0025.xyz = a0039 + TMP40*(waterFogColour.xyz - a0039);
    ret_0 = output0025;
    gl_FragColor = output0025;
    return;
} 