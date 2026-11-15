




vec4 ret_0;
vec4 TMP9;
vec2 c0012;
uniform sampler2D sceneTex;
uniform vec4 pixelOffsetAndBloomScale;

 void main()
{

    vec3 TMP0;

    c0012 = gl_TexCoord[0].xy - pixelOffsetAndBloomScale.xy;
    TMP9 = texture2D(sceneTex, c0012);
    TMP0 = TMP9.w < 9.99899983E-01 ? vec3( 1.00000000E+00, 0.00000000E+00, 1.00000000E+00) : TMP9.xyz;
    ret_0 = vec4(TMP0.x, TMP0.y, TMP0.z, 1.00000000E+00);
    gl_FragColor = ret_0;
    return;
} 