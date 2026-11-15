




vec4 ret_0;
vec4 TMP8;
vec2 c0011;
float x0015;
uniform sampler2D sceneTex;
uniform vec4 params;
uniform vec4 pixelOffsetAndBloomScale;

 void main()
{


    c0011 = gl_TexCoord[0].xy - pixelOffsetAndBloomScale.xy;
    TMP8 = texture2D(sceneTex, c0011);
    x0015 = dot(vec3( 2.12599993E-01, 7.15200007E-01, 7.22000003E-02), TMP8.xyz);
    ret_0 = TMP8*float((x0015 >= params.x));
    gl_FragColor = ret_0;
    return;
} 