




vec4 ret_0;
vec4 TMP10;
vec2 texCoord0011;
vec2 c0013;
vec4 TMP14;
vec2 c0017;
uniform sampler2D sceneTex;
uniform vec4 params;
uniform vec4 pixelOffsetAndBloomScale;
uniform sampler2D bloomTex1;

 void main()
{

    vec4 sceneCol;
    float preLum;
    float postLum;

    texCoord0011 = gl_TexCoord[0].xy*pixelOffsetAndBloomScale.zw;
    c0013 = texCoord0011 - pixelOffsetAndBloomScale.xy;
    TMP10 = texture2D(bloomTex1, c0013);
    c0017 = gl_TexCoord[0].xy - pixelOffsetAndBloomScale.xy;
    TMP14 = texture2D(sceneTex, c0017);
    sceneCol = vec4(TMP14.x, TMP14.y, TMP14.z, 1.00000000E+00);
    preLum = 9.90000010E-01*dot(vec3( 2.12599993E-01, 7.15200007E-01, 7.22000003E-02), sceneCol.xyz) + 9.99999978E-03;
    postLum = (preLum*(1.00000000E+00 + preLum/params.z))/(preLum + 1.00000000E+00);
    ret_0 = sceneCol*(postLum/preLum) + TMP10*params.y;
    gl_FragColor = ret_0;
    return;
} 